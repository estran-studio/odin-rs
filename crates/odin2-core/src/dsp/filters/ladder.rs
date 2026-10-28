//! Ladder Filter (Moog-style)
//!
//! A 4-pole ladder filter with resonance feedback.
//! Based on Will Pirkle's implementation and the Odin 2 LadderFilter.
//!
//! Supports multiple filter modes via Oberheim-style output mixing.

use super::va_one_pole::VAOnePoleFilter;
use super::{Filter, LadderFilterType};
use crate::dsp::math::fast_tan;
use crate::constants::TWO_PI;

/// Maximum resonance value (before self-oscillation)
const MAX_K: f32 = 3.88;

/// Ladder Filter
///
/// A classic 4-pole resonant filter using cascaded VA one-pole filters.
/// The Oberheim variation allows different filter responses (LP2, LP4, BP2, BP4, HP2, HP4).
pub struct LadderFilter {
    /// Four cascaded one-pole filters
    lpf: [VAOnePoleFilter; 4],

    /// Filter type (LP4, LP2, BP4, BP2, HP4, HP2)
    filter_type: LadderFilterType,

    /// Resonance control (k, 0 to ~4)
    k: f32,

    /// Modulated k value
    k_modded: f32,

    /// Gamma for feedback calculation (G^4)
    gamma: f32,

    /// Alpha_0 for feedback scaling
    alpha_0: f32,

    /// Oberheim variation coefficients
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,

    /// Current cutoff frequency
    cutoff: f32,

    /// Sample rate
    sample_rate: f32,

    /// One over sample rate
    one_over_sample_rate: f32,

    /// Last frequency for optimization (skip recalc if unchanged)
    last_freq: f32,

    /// Overdrive/saturation amount (0 to 1)
    overdrive: f32,
}

impl LadderFilter {
    /// Create a new ladder filter
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            lpf: [
                VAOnePoleFilter::new(sample_rate),
                VAOnePoleFilter::new(sample_rate),
                VAOnePoleFilter::new(sample_rate),
                VAOnePoleFilter::new(sample_rate),
            ],
            filter_type: LadderFilterType::LP4,
            k: 0.0,
            k_modded: 0.0,
            gamma: 0.0,
            alpha_0: 1.0,
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 1.0,
            cutoff: 1000.0,
            sample_rate,
            one_over_sample_rate: 1.0 / sample_rate,
            last_freq: -1.0,
            overdrive: 0.0,
        };

        filter.update_coefficients();
        filter
    }

    /// Set filter type
    pub fn set_filter_type(&mut self, filter_type: LadderFilterType) {
        if self.filter_type != filter_type {
            self.filter_type = filter_type;
            self.update_oberheim_coefficients();
            self.last_freq = -1.0; // Force recalculation
        }
    }

    /// Set overdrive amount (0 to 1)
    pub fn set_overdrive(&mut self, overdrive: f32) {
        self.overdrive = overdrive.max(0.0);
    }

    /// Update filter coefficients
    fn update_coefficients(&mut self) {
        // Skip if frequency hasn't changed
        if self.last_freq == self.cutoff {
            return;
        }
        self.last_freq = self.cutoff;

        // Clamp resonance
        self.k_modded = self.k.clamp(0.0, MAX_K);

        // Prewarp for BZT
        let wd = TWO_PI * self.cutoff;
        let wa = 2.0 * self.sample_rate * fast_tan(wd * self.one_over_sample_rate * 0.5);
        let g = wa * self.one_over_sample_rate * 0.5;

        // G - the feedforward coefficient in the VA One Pole
        let big_g = g / (1.0 + g);

        // Set alphas for all stages
        for lpf in &mut self.lpf {
            lpf.alpha = big_g;
        }

        // Set betas for feedback
        self.lpf[0].beta = big_g * big_g * big_g / (1.0 + g);
        self.lpf[1].beta = big_g * big_g / (1.0 + g);
        self.lpf[2].beta = big_g / (1.0 + g);
        self.lpf[3].beta = 1.0 / (1.0 + g);

        // Gamma = G^4
        self.gamma = big_g * big_g * big_g * big_g;

        // Alpha_0 for feedback scaling
        self.alpha_0 = 1.0 / (1.0 + self.k_modded * self.gamma);

        // Update Oberheim coefficients
        self.update_oberheim_coefficients();
    }

    /// Update Oberheim variation coefficients based on filter type
    fn update_oberheim_coefficients(&mut self) {
        match self.filter_type {
            LadderFilterType::LP4 => {
                self.a = 0.0;
                self.b = 0.0;
                self.c = 0.0;
                self.d = 0.0;
                self.e = 1.0;
            }
            LadderFilterType::LP2 => {
                self.a = 0.0;
                self.b = 0.0;
                self.c = 1.0;
                self.d = 0.0;
                self.e = 0.0;
            }
            LadderFilterType::BP4 => {
                self.a = 0.0;
                self.b = 0.0;
                self.c = 4.0;
                self.d = -8.0;
                self.e = 4.0;
            }
            LadderFilterType::BP2 => {
                self.a = 0.0;
                self.b = 2.0;
                self.c = -2.0;
                self.d = 0.0;
                self.e = 0.0;
            }
            LadderFilterType::HP4 => {
                self.a = 1.0;
                self.b = -4.0;
                self.c = 6.0;
                self.d = -4.0;
                self.e = 1.0;
            }
            LadderFilterType::HP2 => {
                self.a = 1.0;
                self.b = -2.0;
                self.c = 1.0;
                self.d = 0.0;
                self.e = 0.0;
            }
        }
    }

    /// Apply overdrive/saturation to output
    #[inline]
    fn apply_overdrive(&self, input: f32) -> f32 {
        super::saturate(input, self.overdrive, 3.5)
    }
}

impl Filter for LadderFilter {
    fn process(&mut self, input: f32) -> f32 {
        // Update coefficients if needed
        self.update_coefficients();

        // Calculate sigma (sum of feedback outputs)
        let sigma = self.lpf[0].get_feedback_output()
            + self.lpf[1].get_feedback_output()
            + self.lpf[2].get_feedback_output()
            + self.lpf[3].get_feedback_output();

        // Calculate input to first filter (with resonance feedback)
        let u = (input - self.k_modded * sigma) * self.alpha_0;

        // Cascade through 4 filters
        let lp1 = self.lpf[0].process(u);
        let lp2 = self.lpf[1].process(lp1);
        let lp3 = self.lpf[2].process(lp2);
        let lp4 = self.lpf[3].process(lp3);

        // Oberheim variation mixing
        let output = self.a * u + self.b * lp1 + self.c * lp2 + self.d * lp3 + self.e * lp4;

        // Apply overdrive
        self.apply_overdrive(output)
    }

    fn set_cutoff(&mut self, cutoff: f32) {
        // Clamp to valid range
        self.cutoff = cutoff.clamp(20.0, 20000.0);
    }

    fn set_resonance(&mut self, resonance: f32) {
        // Map 0-1 to 0-MAX_K
        self.k = resonance * MAX_K;
        self.k_modded = self.k;
        self.last_freq = -1.0; // Force coefficient recalculation
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.one_over_sample_rate = 1.0 / sample_rate;

        for lpf in &mut self.lpf {
            lpf.set_sample_rate(sample_rate);
        }

        self.last_freq = -1.0; // Force recalculation
    }

    fn reset(&mut self) {
        for lpf in &mut self.lpf {
            lpf.reset();
        }
        self.last_freq = -1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ladder_filter_lp4() {
        let mut filter = LadderFilter::new(44100.0);
        filter.set_cutoff(1000.0);
        filter.set_resonance(0.5);
        filter.set_filter_type(LadderFilterType::LP4);

        // Process white noise-ish signal
        let mut output_sum = 0.0;
        for i in 0..1000 {
            let input = if i % 2 == 0 { 1.0 } else { -1.0 };
            output_sum += filter.process(input).abs();
        }

        // LP should attenuate high frequencies
        assert!(output_sum < 1000.0, "LP4 should attenuate: {}", output_sum);
    }

    #[test]
    fn test_ladder_filter_resonance() {
        let mut filter = LadderFilter::new(44100.0);
        filter.set_cutoff(440.0);
        filter.set_filter_type(LadderFilterType::LP4);

        // Test with low resonance
        filter.set_resonance(0.1);
        let mut low_res_output = 0.0;
        for _ in 0..100 {
            low_res_output = filter.process(1.0);
        }

        filter.reset();

        // Test with high resonance
        filter.set_resonance(0.9);
        let mut high_res_output = 0.0;
        for _ in 0..100 {
            high_res_output = filter.process(1.0);
        }

        // Both should produce valid output
        assert!(low_res_output.is_finite());
        assert!(high_res_output.is_finite());
    }

    #[test]
    fn test_ladder_filter_types() {
        let mut filter = LadderFilter::new(44100.0);
        filter.set_cutoff(1000.0);
        filter.set_resonance(0.3);

        // Test all filter types produce valid output
        for filter_type in [
            LadderFilterType::LP4,
            LadderFilterType::LP2,
            LadderFilterType::BP4,
            LadderFilterType::BP2,
            LadderFilterType::HP4,
            LadderFilterType::HP2,
        ] {
            filter.set_filter_type(filter_type);
            filter.reset();

            for _ in 0..100 {
                let output = filter.process(1.0);
                assert!(output.is_finite(), "Filter type {:?} produced invalid output", filter_type);
            }
        }
    }
}
