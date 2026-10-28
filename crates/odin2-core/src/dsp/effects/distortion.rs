//! Distorsion polyphonique Odin 2, après amplificateur et avant enveloppe.
//!
//! Traduction de `Source/audio/FX/OversamplingDistortion.cpp:26-179` (GPL-3.0-or-later).
//! Le cas Sine tombe volontairement dans Cube dans la source C++.

use crate::preset::{DistortionAlgo, DistortionParams};

pub struct OversamplingDistortion {
    algorithm: DistortionAlgo,
    last_input: f64,
    bias: f32,
    threshold: f32,
    threshold_smooth: f32,
    dry_wet: f32,
    boost_mod: f32,
    mix_mod: f32,
    xv: [f64; 10],
    yv: [f64; 10],
}
impl OversamplingDistortion {
    pub fn new() -> Self {
        Self {
            algorithm: DistortionAlgo::Clamp, last_input: 0.0, bias: 0.0,
            threshold: 0.343, threshold_smooth: 0.343, dry_wet: 1.0,
            boost_mod: 0.0, mix_mod: 0.0, xv: [0.0; 10], yv: [0.0; 10],
        }
    }
    pub fn configure(&mut self, params: &DistortionParams) {
        self.algorithm = params.algorithm;
        self.set_threshold(params.boost);
        self.set_dry_wet(params.dry_wet);
    }
    pub fn set_algorithm(&mut self, algorithm: DistortionAlgo) { self.algorithm = algorithm; }
    pub fn set_threshold(&mut self, boost: f32) {
        let t = 1.0 - boost;
        self.threshold = t * t * t;
    }
    pub fn set_bias(&mut self, bias: f32) { self.bias = bias; }
    pub fn set_dry_wet(&mut self, mix: f32) { self.dry_wet = mix; }
    pub fn set_modulation(&mut self, boost: f32, dry_wet: f32) {
        self.boost_mod = boost;
        self.mix_mod = dry_wet;
    }
    /// Le filtre de décimation amont utilise les mêmes coefficients à toute fréquence.
    pub fn set_sample_rate(&mut self, _sample_rate: f32) {}
    pub fn reset(&mut self) {
        self.xv.fill(0.0);
        self.yv.fill(0.0);
        self.threshold_smooth = self.threshold;
        // L'amont conserve m_last_input lors du reset.
    }
    fn shape(&self, mut sample: f64, threshold: f32) -> f64 {
        let t = threshold as f64;
        let bias = self.bias as f64;
        match self.algorithm {
            DistortionAlgo::Clamp => sample.clamp(bias - t, bias + t),
            DistortionAlgo::Zero => {
                if sample > bias + t || sample < bias - t { 0.0 } else { sample }
            }
            DistortionAlgo::Sine => {
                let s = libm::sin(sample);
                s * s * s
            }
            DistortionAlgo::Cube => sample * sample * sample,
            DistortionAlgo::Fold => {
                while libm::fabs(sample) > t {
                    sample = if sample > t { 2.0 * t - sample } else { -2.0 * t - sample };
                }
                sample
            }
        }
    }
    fn filter_step(&mut self, input: f64) {
        for i in 0..9 {
            self.xv[i] = self.xv[i + 1];
            self.yv[i] = self.yv[i + 1];
        }
        self.xv[9] = input * 0.019966841051093;
        self.yv[9] = (self.xv[0] + self.xv[9])
            + 9.0 * (self.xv[1] + self.xv[8])
            + 36.0 * (self.xv[2] + self.xv[7])
            + 84.0 * (self.xv[3] + self.xv[6])
            + 126.0 * (self.xv[4] + self.xv[5])
            - 0.0003977153 * self.yv[0] - 0.0064474617 * self.yv[1]
            - 0.0476997403 * self.yv[2] - 0.2185829743 * self.yv[3]
            - 0.6649234123 * self.yv[4] - 1.4773657709 * self.yv[5]
            - 2.2721421641 * self.yv[6] - 2.6598673212 * self.yv[7]
            - 1.8755960587 * self.yv[8];
    }
    pub fn process(&mut self, input: f32) -> f32 {
        let input = input as f64;
        let upsampled = [
            0.66666666 * self.last_input + 0.33333333 * input,
            0.33333333 * self.last_input + 0.66666666 * input, input,
        ];
        self.last_input = input;
        self.threshold_smooth = self.threshold_smooth * 0.998 + (1.0 - 0.998) * self.threshold;
        let mut threshold = ((self.threshold_smooth - self.boost_mod) * (1.0 - 0.05) + 0.05).clamp(0.05, 1.0);
        if self.algorithm == DistortionAlgo::Zero { threshold = 0.5 + threshold * 0.5; }
        for sample in upsampled { self.filter_step(self.shape(sample, threshold)); }
        let mix = (self.dry_wet + self.mix_mod).clamp(0.0, 1.0);
        let wet = match self.algorithm {
            DistortionAlgo::Clamp | DistortionAlgo::Fold | DistortionAlgo::Zero =>
                self.yv[9] * mix as f64 / threshold as f64,
            _ => self.yv[9] * mix as f64,
        };
        (wet + input * (1.0 - mix) as f64) as f32
    }
}
impl Default for OversamplingDistortion {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sine_includes_upstream_cubic_fallthrough() {
        let mut d = OversamplingDistortion::new();
        d.set_algorithm(DistortionAlgo::Sine);
        let expected = libm::sin(0.7_f64);
        assert_eq!(d.shape(0.7, 0.1), expected * expected * expected);
    }
    #[test]
    fn fold_has_no_ten_reflection_limit() {
        let mut d = OversamplingDistortion::new();
        d.set_algorithm(DistortionAlgo::Fold);
        assert!(d.shape(5.0, 0.05).abs() <= 0.05_f32 as f64);
    }
    #[test]
    fn zero_threshold_is_used_for_output_compensation() {
        let mut d = OversamplingDistortion::new();
        d.set_algorithm(DistortionAlgo::Zero);
        d.set_threshold(1.0);
        d.reset();
        for _ in 0..1000 { d.process(0.1); }
        assert!((d.process(0.1) - 0.1 / 0.525).abs() < 1e-4);
    }
}
