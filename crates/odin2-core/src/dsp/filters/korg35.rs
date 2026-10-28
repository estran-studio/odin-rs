//! Korg35 Filter
//!
//! A 12dB/octave lowpass/highpass filter based on the Korg MS-20 topology.
//! Based on Will Pirkle's implementation from "Designing Software Synthesizer Plug-Ins in C++".

use super::{Filter, VAOnePoleFilter};
use crate::dsp::math::fast_tan;

/// Filter frequency limits
const FILTER_FC_MIN: f32 = 20.0;
const FILTER_FC_MAX: f32 = 20000.0;

/// Korg35 filter
///
/// A 2-pole (12dB/oct) filter with a distinctive aggressive resonance character.
/// Can operate as either lowpass or highpass.
pub struct Korg35Filter {
    /// Lowpass filter stage 1
    lpf1: VAOnePoleFilter,
    /// Lowpass filter stage 2
    lpf2: VAOnePoleFilter,
    /// Highpass filter stage 1
    hpf1: VAOnePoleFilter,
    /// Highpass filter stage 2
    hpf2: VAOnePoleFilter,

    /// Sample rate
    sample_rate: f32,
    /// One over sample rate
    one_over_sample_rate: f32,

    /// Cutoff frequency
    cutoff: f32,
    /// Resonance amount (k: 0.01-1.96)
    k: f32,
    /// Alpha coefficient
    alpha: f32,

    /// Whether this is a lowpass (true) or highpass (false) filter
    is_lowpass: bool,

    /// Last frequency for optimization
    last_freq: f32,

    /// Overdrive amount
    overdrive: f32,
}

impl Korg35Filter {
    /// Create a new Korg35 filter
    pub fn new(sample_rate: f32) -> Self {
        let mut lpf1 = VAOnePoleFilter::new(sample_rate);
        let mut lpf2 = VAOnePoleFilter::new(sample_rate);
        let mut hpf1 = VAOnePoleFilter::new(sample_rate);
        let mut hpf2 = VAOnePoleFilter::new(sample_rate);

        lpf1.set_lowpass();
        lpf2.set_lowpass();
        hpf1.set_highpass();
        hpf2.set_highpass();

        Self {
            lpf1,
            lpf2,
            hpf1,
            hpf2,
            sample_rate,
            one_over_sample_rate: 1.0 / sample_rate,
            cutoff: 1000.0,
            k: 0.01,
            alpha: 1.0,
            is_lowpass: true,
            last_freq: -1.0,
            overdrive: 0.0,
        }
    }

    /// Set to lowpass mode
    pub fn set_lowpass(&mut self) {
        self.is_lowpass = true;
        self.last_freq = -1.0; // Force recalculation
    }

    /// Set to highpass mode
    pub fn set_highpass(&mut self) {
        self.is_lowpass = false;
        self.last_freq = -1.0; // Force recalculation
    }

    /// Check if in lowpass mode
    pub fn is_lowpass(&self) -> bool {
        self.is_lowpass
    }

    /// Set overdrive amount (0.0 to 1.0)
    pub fn set_overdrive(&mut self, amount: f32) {
        self.overdrive = amount.max(0.0);
    }

    /// Apply overdrive saturation
    fn apply_overdrive(&self, input: f32) -> f32 {
        super::saturate(input, self.overdrive, 3.0)
    }

    /// Update filter coefficients
    fn update_coefficients(&mut self) {
        if self.cutoff == self.last_freq {
            return;
        }
        self.last_freq = self.cutoff;

        let freq = self.cutoff.clamp(FILTER_FC_MIN, FILTER_FC_MAX);

        // BZT (Bilinear Z-Transform)
        let wd = 2.0 * core::f32::consts::PI * freq;
        let wa = 2.0 * self.sample_rate * fast_tan(wd * self.one_over_sample_rate * 0.5);
        let g = wa * self.one_over_sample_rate * 0.5;
        let big_g = g / (1.0 + g);

        // Set alpha for all stages
        self.lpf1.set_alpha(big_g);
        self.lpf2.set_alpha(big_g);
        self.hpf1.set_alpha(big_g);
        self.hpf2.set_alpha(big_g);

        // Calculate feedback alpha
        self.alpha = 1.0 / (1.0 - self.k * big_g + self.k * big_g * big_g);

        // Set beta coefficients based on filter type
        if self.is_lowpass {
            self.lpf2.set_beta((self.k - self.k * big_g) / (1.0 + g));
            self.hpf1.set_beta(-1.0 / (1.0 + g));
        } else {
            self.hpf2.set_beta(-big_g / (1.0 + g));
            self.lpf1.set_beta(1.0 / (1.0 + g));
        }
    }
}

impl Filter for Korg35Filter {
    fn process(&mut self, input: f32) -> f32 {
        self.update_coefficients();

        let output = if self.is_lowpass {
            // Lowpass path
            let y1 = self.lpf1.process(input);
            let s35 = self.lpf2.get_feedback_output() + self.hpf1.get_feedback_output();
            let u = self.alpha * (y1 + s35);

            let y = self.k * self.lpf2.process(u);
            self.hpf1.process(y);
            y / self.k
        } else {
            // Highpass path
            let y1 = self.hpf1.process(input);
            let s35 = self.hpf2.get_feedback_output() + self.lpf1.get_feedback_output();
            let u = self.alpha * (y1 + s35);

            let y = self.k * u;
            self.lpf1.process(self.hpf2.process(y));
            y / self.k
        };

        self.apply_overdrive(output)
    }

    fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.clamp(FILTER_FC_MIN, FILTER_FC_MAX);
    }

    fn set_resonance(&mut self, res: f32) {
        // Map resonance (0-1) to k (0.01-1.96)
        // Note: original uses 1.95 to avoid self-oscillation
        self.k = res.clamp(0.0, 1.0) * 1.95 + 0.01;
        self.last_freq = -1.0; // Force recalculation
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.one_over_sample_rate = 1.0 / sample_rate;
        self.lpf1.set_sample_rate(sample_rate);
        self.lpf2.set_sample_rate(sample_rate);
        self.hpf1.set_sample_rate(sample_rate);
        self.hpf2.set_sample_rate(sample_rate);
        self.last_freq = -1.0; // Force recalculation
    }

    fn reset(&mut self) {
        self.lpf1.reset();
        self.lpf2.reset();
        self.hpf1.reset();
        self.hpf2.reset();
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_korg35_basic() {
        let mut filter = Korg35Filter::new(44100.0);
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
    fn test_korg35_lowpass_vs_highpass() {
        let sample_rate = 44100.0;
        let cutoff = 1000.0;

        // Generate test signal above cutoff
        let high_freq = 5000.0;

        let mut filter_lp = Korg35Filter::new(sample_rate);
        filter_lp.set_cutoff(cutoff);
        filter_lp.set_lowpass();
        filter_lp.set_resonance(0.3);

        let mut filter_hp = Korg35Filter::new(sample_rate);
        filter_hp.set_cutoff(cutoff);
        filter_hp.set_highpass();
        filter_hp.set_resonance(0.3);

        // Process high frequency through both
        let mut lp_energy = 0.0f32;
        let mut hp_energy = 0.0f32;

        for i in 0..2000 {
            let t = i as f32 / sample_rate;
            let input = libm::sinf(2.0 * core::f32::consts::PI * high_freq * t);

            let lp_out = filter_lp.process(input);
            let hp_out = filter_hp.process(input);

            // Skip first samples for settling
            if i > 100 {
                lp_energy += lp_out * lp_out;
                hp_energy += hp_out * hp_out;
            }
        }

        // HP should pass more of the high frequency than LP
        assert!(
            hp_energy > lp_energy,
            "HP should pass high frequencies better: HP={}, LP={}",
            hp_energy,
            lp_energy
        );
    }

    #[test]
    fn test_korg35_resonance() {
        let mut filter = Korg35Filter::new(44100.0);
        filter.set_cutoff(1000.0);

        // Low resonance
        filter.set_resonance(0.0);
        let low_res: Vec<f32> = (0..100).map(|_| filter.process(0.5)).collect();

        filter.reset();

        // High resonance
        filter.set_resonance(0.9);
        let high_res: Vec<f32> = (0..100).map(|_| filter.process(0.5)).collect();

        // Both should produce valid output
        let low_max = low_res.iter().map(|x: &f32| x.abs()).fold(0.0f32, f32::max);
        let high_max = high_res.iter().map(|x: &f32| x.abs()).fold(0.0f32, f32::max);

        assert!(low_max.is_finite());
        assert!(high_max.is_finite());
    }
}
