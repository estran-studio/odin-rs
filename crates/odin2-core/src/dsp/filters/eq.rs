//! Biquad EQ Filter
//!
//! A parametric EQ band using biquad filter coefficients.
//! Based on the Audio EQ Cookbook by Robert Bristow-Johnson.

use crate::dsp::filters::BiquadFilter;
use core::f32::consts::PI;

/// Biquad Parametric EQ Band
///
/// A peaking EQ filter that can boost or cut frequencies around a center frequency.
/// Uses biquad filter coefficients from the Audio EQ Cookbook.
pub struct BiquadEQ {
    /// Internal biquad filter
    filter: BiquadFilter,

    /// Q factor (bandwidth)
    q: f32,

    /// Center frequency in Hz
    freq: f32,

    /// Gain in dB
    gain: f32,
}

impl BiquadEQ {
    /// Create a new biquad EQ
    pub fn new(sample_rate: f32) -> Self {
        let mut eq = Self {
            filter: BiquadFilter::new(sample_rate),
            q: 1.0,
            freq: 1000.0,
            gain: 0.0,
        };
        eq.recalculate_coefficients();
        eq
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.filter.set_sample_rate(sample_rate);
        self.recalculate_coefficients();
    }

    /// Set Q factor (bandwidth)
    /// Higher Q = narrower bandwidth
    pub fn set_q(&mut self, q: f32) {
        self.q = q.max(0.1);
        self.recalculate_coefficients();
    }

    /// Set center frequency in Hz
    pub fn set_freq(&mut self, freq: f32) {
        self.freq = freq.clamp(20.0, self.filter.sample_rate() * 0.49);
        self.recalculate_coefficients();
    }

    /// Set gain in dB (-24 to +24 typical)
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(-40.0, 40.0);
        self.recalculate_coefficients();
    }

    /// Set all EQ parameters at once
    pub fn set_params(&mut self, q: f32, freq: f32, gain: f32) {
        self.q = q.max(0.1);
        self.freq = freq.clamp(20.0, self.filter.sample_rate() * 0.49);
        self.gain = gain.clamp(-40.0, 40.0);
        self.recalculate_coefficients();
    }

    /// Recalculate biquad coefficients
    /// Based on Audio EQ Cookbook: https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html
    fn recalculate_coefficients(&mut self) {
        let w0 = 2.0 * PI * self.freq * self.filter.one_over_sample_rate();
        let sin_w0 = libm::sinf(w0);
        let cos_w0 = libm::cosf(w0);

        let alpha = sin_w0 / (2.0 * self.q);

        // A = 10^(dBgain/40) = sqrt(10^(dBgain/20))
        let a = libm::powf(10.0, self.gain / 40.0);

        // Peaking EQ coefficients (unnormalized)
        let a0 = 1.0 + alpha / a;
        let a0_inv = 1.0 / a0;

        self.filter.b0 = (1.0 + alpha * a) * a0_inv;
        self.filter.b1 = -2.0 * cos_w0 * a0_inv;
        self.filter.b2 = (1.0 - alpha * a) * a0_inv;
        self.filter.a1 = self.filter.b1; // Same as b1 for peaking EQ
        self.filter.a2 = (1.0 - alpha / a) * a0_inv;
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.filter.reset();
    }

    /// Process one sample
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        self.filter.process(input)
    }
}

/// EQ band type for multi-band EQ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EQBandType {
    /// Low shelf filter
    LowShelf,
    /// High shelf filter
    HighShelf,
    /// Peaking/parametric band
    #[default]
    Peaking,
    /// Low pass filter
    LowPass,
    /// High pass filter
    HighPass,
}

/// Flexible EQ band that supports different filter types
pub struct EQBand {
    /// Internal biquad filter
    filter: BiquadFilter,

    /// Band type
    band_type: EQBandType,

    /// Q factor (bandwidth)
    q: f32,

    /// Center/corner frequency in Hz
    freq: f32,

    /// Gain in dB (for shelf and peaking only)
    gain: f32,
}

impl EQBand {
    /// Create a new EQ band
    pub fn new(sample_rate: f32) -> Self {
        let mut band = Self {
            filter: BiquadFilter::new(sample_rate),
            band_type: EQBandType::Peaking,
            q: 1.0,
            freq: 1000.0,
            gain: 0.0,
        };
        band.recalculate_coefficients();
        band
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.filter.set_sample_rate(sample_rate);
        self.recalculate_coefficients();
    }

    /// Set band type
    pub fn set_band_type(&mut self, band_type: EQBandType) {
        self.band_type = band_type;
        self.recalculate_coefficients();
    }

    /// Set Q factor
    pub fn set_q(&mut self, q: f32) {
        self.q = q.max(0.1);
        self.recalculate_coefficients();
    }

    /// Set frequency
    pub fn set_freq(&mut self, freq: f32) {
        self.freq = freq.clamp(20.0, self.filter.sample_rate() * 0.49);
        self.recalculate_coefficients();
    }

    /// Set gain in dB
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(-40.0, 40.0);
        self.recalculate_coefficients();
    }

    /// Recalculate coefficients based on band type
    fn recalculate_coefficients(&mut self) {
        let w0 = 2.0 * PI * self.freq * self.filter.one_over_sample_rate();
        let sin_w0 = libm::sinf(w0);
        let cos_w0 = libm::cosf(w0);
        let alpha = sin_w0 / (2.0 * self.q);

        match self.band_type {
            EQBandType::Peaking => {
                let a = libm::powf(10.0, self.gain / 40.0);
                let a0 = 1.0 + alpha / a;
                let a0_inv = 1.0 / a0;

                self.filter.b0 = (1.0 + alpha * a) * a0_inv;
                self.filter.b1 = -2.0 * cos_w0 * a0_inv;
                self.filter.b2 = (1.0 - alpha * a) * a0_inv;
                self.filter.a1 = self.filter.b1;
                self.filter.a2 = (1.0 - alpha / a) * a0_inv;
            }
            EQBandType::LowShelf => {
                let a = libm::powf(10.0, self.gain / 40.0);
                let sqrt_a = libm::sqrtf(a);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a0_inv = 1.0 / a0;

                self.filter.b0 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha)) * a0_inv;
                self.filter.b1 = (2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0)) * a0_inv;
                self.filter.b2 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha)) * a0_inv;
                self.filter.a1 = (-2.0 * ((a - 1.0) + (a + 1.0) * cos_w0)) * a0_inv;
                self.filter.a2 = ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha) * a0_inv;
            }
            EQBandType::HighShelf => {
                let a = libm::powf(10.0, self.gain / 40.0);
                let sqrt_a = libm::sqrtf(a);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a0_inv = 1.0 / a0;

                self.filter.b0 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha)) * a0_inv;
                self.filter.b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0)) * a0_inv;
                self.filter.b2 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha)) * a0_inv;
                self.filter.a1 = (2.0 * ((a - 1.0) - (a + 1.0) * cos_w0)) * a0_inv;
                self.filter.a2 = ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha) * a0_inv;
            }
            EQBandType::LowPass => {
                let a0 = 1.0 + alpha;
                let a0_inv = 1.0 / a0;

                self.filter.b0 = ((1.0 - cos_w0) / 2.0) * a0_inv;
                self.filter.b1 = (1.0 - cos_w0) * a0_inv;
                self.filter.b2 = ((1.0 - cos_w0) / 2.0) * a0_inv;
                self.filter.a1 = (-2.0 * cos_w0) * a0_inv;
                self.filter.a2 = (1.0 - alpha) * a0_inv;
            }
            EQBandType::HighPass => {
                let a0 = 1.0 + alpha;
                let a0_inv = 1.0 / a0;

                self.filter.b0 = ((1.0 + cos_w0) / 2.0) * a0_inv;
                self.filter.b1 = (-(1.0 + cos_w0)) * a0_inv;
                self.filter.b2 = ((1.0 + cos_w0) / 2.0) * a0_inv;
                self.filter.a1 = (-2.0 * cos_w0) * a0_inv;
                self.filter.a2 = (1.0 - alpha) * a0_inv;
            }
        }
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.filter.reset();
    }

    /// Process one sample
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        self.filter.process(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_eq_basic() {
        let mut eq = BiquadEQ::new(44100.0);
        eq.set_freq(1000.0);
        eq.set_q(1.0);
        eq.set_gain(6.0); // +6dB boost

        for i in 0..1000 {
            let t = i as f32 / 44100.0;
            let input = libm::sinf(2.0 * PI * 440.0 * t);
            let output = eq.process(input);
            assert!(output.is_finite(), "BiquadEQ produced invalid output");
        }
    }

    #[test]
    fn test_biquad_eq_cut() {
        let mut eq = BiquadEQ::new(44100.0);
        eq.set_freq(1000.0);
        eq.set_q(2.0);
        eq.set_gain(-12.0); // -12dB cut

        for i in 0..1000 {
            let t = i as f32 / 44100.0;
            let input = libm::sinf(2.0 * PI * 1000.0 * t);
            let output = eq.process(input);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn test_eq_band_types() {
        let band_types = [
            EQBandType::Peaking,
            EQBandType::LowShelf,
            EQBandType::HighShelf,
            EQBandType::LowPass,
            EQBandType::HighPass,
        ];

        for band_type in band_types {
            let mut band = EQBand::new(44100.0);
            band.set_band_type(band_type);
            band.set_freq(1000.0);
            band.set_q(1.0);
            band.set_gain(6.0);

            for i in 0..1000 {
                let t = i as f32 / 44100.0;
                let input = libm::sinf(2.0 * PI * 440.0 * t);
                let output = band.process(input);
                assert!(
                    output.is_finite(),
                    "EQBand {:?} produced invalid output",
                    band_type
                );
            }
        }
    }

    #[test]
    fn test_eq_zero_gain() {
        let mut eq = BiquadEQ::new(44100.0);
        eq.set_freq(1000.0);
        eq.set_q(1.0);
        eq.set_gain(0.0); // Unity gain

        // With zero gain, output should be approximately equal to input
        // (after filter settles)
        let mut eq2 = eq;
        for _ in 0..100 {
            let _ = eq2.process(0.5);
        }

        let input = 0.5;
        let output = eq2.process(input);
        // Should be close to input at steady state with 0 dB gain
        assert!(
            (output - input).abs() < 0.1,
            "Zero gain should pass signal through"
        );
    }
}
