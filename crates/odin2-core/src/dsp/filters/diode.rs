//! Diode Ladder Filter
//!
//! A non-linear 4-pole lowpass filter based on diode ladder topology.
//! Based on the implementation from Will Pirkle's "Designing Software Synthesizer Plug-Ins in C++".

use super::{Filter, VAOnePoleFilter};
use crate::dsp::math::fast_tan;

/// Filter frequency limits
const FILTER_FC_MIN: f32 = 20.0;
const FILTER_FC_MAX: f32 = 20000.0;

/// Diode ladder filter
///
/// A 4-pole (24dB/oct) lowpass filter with a characteristic "fat" sound
/// due to the non-linear saturation in the diode ladder topology.
pub struct DiodeFilter {
    /// Four VA one-pole filter stages
    lpf: [VAOnePoleFilter; 4],

    /// Sample rate
    sample_rate: f32,

    /// One over sample rate
    one_over_sample_rate: f32,

    /// Cutoff frequency
    cutoff: f32,

    /// Resonance amount (k: 0-16)
    k: f32,

    /// Gamma coefficient for feedback
    gamma: f32,

    /// Feedback scalars
    sg1: f32,
    sg2: f32,
    sg3: f32,
    sg4: f32,

    /// Last frequency for optimization
    last_freq: f32,

    /// Overdrive amount
    overdrive: f32,
}

impl DiodeFilter {
    /// Create a new diode filter
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            lpf: [
                VAOnePoleFilter::new(sample_rate),
                VAOnePoleFilter::new(sample_rate),
                VAOnePoleFilter::new(sample_rate),
                VAOnePoleFilter::new(sample_rate),
            ],
            sample_rate,
            one_over_sample_rate: 1.0 / sample_rate,
            cutoff: 1000.0,
            k: 0.0,
            gamma: 0.0,
            sg1: 0.0,
            sg2: 0.0,
            sg3: 0.0,
            sg4: 0.0,
            last_freq: -1.0,
            overdrive: 0.0,
        };

        // Set all stages to lowpass
        for lpf in &mut filter.lpf {
            lpf.set_lowpass();
        }

        filter
    }

    /// Update filter coefficients
    fn update_coefficients(&mut self) {
        if self.cutoff == self.last_freq {
            return;
        }
        self.last_freq = self.cutoff;

        let freq = self.cutoff.clamp(FILTER_FC_MIN, FILTER_FC_MAX);

        // Calculate alpha using bilinear transform
        let wd = 2.0 * core::f32::consts::PI * freq;
        let wa = 2.0 * self.sample_rate * fast_tan(wd * self.one_over_sample_rate * 0.5);
        let g = wa * self.one_over_sample_rate * 0.5;

        // Calculate G values for each stage (diode ladder specific)
        let g4 = 0.5 * g / (1.0 + g);
        let g3 = 0.5 * g / (1.0 + g - 0.5 * g * g4);
        let g2 = 0.5 * g / (1.0 + g - 0.5 * g * g3);
        let g1 = g / (1.0 + g - g * g2);

        self.gamma = g4 * g3 * g2 * g1;

        // Feedback scalars
        self.sg1 = g4 * g3 * g2;
        self.sg2 = g4 * g3;
        self.sg3 = g4;
        self.sg4 = 1.0;

        let gg = g / (1.0 + g);

        // Update each filter stage
        self.lpf[0].set_alpha(gg);
        self.lpf[1].set_alpha(gg);
        self.lpf[2].set_alpha(gg);
        self.lpf[3].set_alpha(gg);

        self.lpf[0].set_beta(1.0 / (1.0 + g - g * g2));
        self.lpf[1].set_beta(1.0 / (1.0 + g - 0.5 * g * g3));
        self.lpf[2].set_beta(1.0 / (1.0 + g - 0.5 * g * g4));
        self.lpf[3].set_beta(1.0 / (1.0 + g));

        self.lpf[0].set_delta(g);
        self.lpf[1].set_delta(0.5 * g);
        self.lpf[2].set_delta(0.5 * g);
        self.lpf[3].set_delta(0.0);

        self.lpf[0].set_gamma(1.0 + g1 * g2);
        self.lpf[1].set_gamma(1.0 + g2 * g3);
        self.lpf[2].set_gamma(1.0 + g3 * g4);
        self.lpf[3].set_gamma(1.0);

        self.lpf[0].set_epsilon(g2);
        self.lpf[1].set_epsilon(g3);
        self.lpf[2].set_epsilon(g4);
        self.lpf[3].set_epsilon(0.0);

        self.lpf[0].set_a0(1.0);
        self.lpf[1].set_a0(0.5);
        self.lpf[2].set_a0(0.5);
        self.lpf[3].set_a0(0.5);
    }

    /// Set overdrive amount (0.0 to 1.0)
    pub fn set_overdrive(&mut self, amount: f32) {
        self.overdrive = amount.max(0.0);
    }

    /// Apply saturation
    fn apply_overdrive(&self, input: f32) -> f32 {
        super::saturate(input, self.overdrive, 3.5)
    }
}

impl Filter for DiodeFilter {
    fn process(&mut self, input: f32) -> f32 {
        self.update_coefficients();

        // Get feedback outputs
        self.lpf[3].set_feedback(0.0);
        self.lpf[2].set_feedback(self.lpf[3].get_feedback_output());
        self.lpf[1].set_feedback(self.lpf[2].get_feedback_output());
        self.lpf[0].set_feedback(self.lpf[1].get_feedback_output());

        // Calculate sigma (sum of weighted feedback outputs)
        let sigma = self.sg1 * self.lpf[0].get_feedback_output()
            + self.sg2 * self.lpf[1].get_feedback_output()
            + self.sg3 * self.lpf[2].get_feedback_output()
            + self.sg4 * self.lpf[3].get_feedback_output();

        // Apply resonance compensation for passband gain
        let input_scaled = input * (1.0 + 0.3 * self.k);

        // Calculate input to filter cascade
        let u = (input_scaled - self.k * sigma) / (1.0 + self.k * self.gamma);

        // Process through all four stages
        let mut output = self.lpf[0].process(u);
        output = self.lpf[1].process(output);
        output = self.lpf[2].process(output);
        output = self.lpf[3].process(output);

        // Apply overdrive
        self.apply_overdrive(output)
    }

    fn set_cutoff(&mut self, freq: f32) {
        self.cutoff = freq.clamp(FILTER_FC_MIN, FILTER_FC_MAX);
    }

    fn set_resonance(&mut self, res: f32) {
        // Resonance maps to k (0-16 range)
        self.k = res.clamp(0.0, 1.0) * 16.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.one_over_sample_rate = 1.0 / sample_rate;
        for lpf in &mut self.lpf {
            lpf.set_sample_rate(sample_rate);
        }
        self.last_freq = -1.0; // Force coefficient recalculation
    }

    fn reset(&mut self) {
        for lpf in &mut self.lpf {
            lpf.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_diode_basic() {
        let mut filter = DiodeFilter::new(44100.0);
        filter.set_cutoff(1000.0);
        filter.set_resonance(0.5);

        // Process some samples
        for i in 0..1000 {
            let input = if i % 100 < 50 { 1.0 } else { -1.0 }; // Square wave
            let output = filter.process(input);
            assert!(output.is_finite(), "Filter produced invalid output");
        }
    }

    #[test]
    fn test_diode_resonance() {
        let mut filter = DiodeFilter::new(44100.0);
        filter.set_cutoff(1000.0);

        // Low resonance
        filter.set_resonance(0.0);
        let low_res: Vec<f32> = (0..100)
            .map(|_| filter.process(0.5))
            .collect();

        filter.reset();

        // High resonance
        filter.set_resonance(0.9);
        let high_res: Vec<f32> = (0..100)
            .map(|_| filter.process(0.5))
            .collect();

        // High resonance should produce different (likely larger) output
        let low_max = low_res.iter().map(|x: &f32| x.abs()).fold(0.0f32, f32::max);
        let high_max = high_res.iter().map(|x: &f32| x.abs()).fold(0.0f32, f32::max);

        // Both should produce valid output
        assert!(low_max.is_finite());
        assert!(high_max.is_finite());
    }
}
