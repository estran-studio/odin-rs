//! Biquad Filter
//!
//! A general purpose biquad (bi-quadratic) filter using Direct Form II
//! transposed structure. Used as a building block for allpass, resonator,
//! and other filter types.

use core::f32::consts::PI;

/// `juce::dsp::FastMathApproximations::cos<double>`: Padé valable sur [-π, π].
fn juce_cos(x: f64) -> f64 {
    let x2 = x * x;
    let numerator = -(-39251520.0 + x2 * (18471600.0 + x2 * (-1075032.0 + 14615.0 * x2)));
    let denominator = 39251520.0 + x2 * (1154160.0 + x2 * (16632.0 + x2 * 127.0));
    numerator / denominator
}

/// `juce::dsp::FastMathApproximations::cos<float>`.
fn juce_cos_f32(x: f32) -> f32 {
    let x2 = x * x;
    let numerator = -(-39251520.0 + x2 * (18471600.0 + x2 * (-1075032.0 + 14615.0 * x2)));
    let denominator = 39251520.0 + x2 * (1154160.0 + x2 * (16632.0 + x2 * 127.0));
    numerator / denominator
}

/// Biquad filter using Direct Form II transposed
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    /// Feed-forward coefficient b0
    pub b0: f32,
    /// Feed-forward coefficient b1
    pub b1: f32,
    /// Feed-forward coefficient b2
    pub b2: f32,
    /// Feedback coefficient a1
    pub a1: f32,
    /// Feedback coefficient a2
    pub a2: f32,

    /// Delay element z1
    z1: f32,
    /// Delay element z2
    z2: f32,

    /// Sample rate
    sample_rate: f32,
    /// One over sample rate
    one_over_sample_rate: f32,
}

impl BiquadFilter {
    /// Create a new biquad filter
    pub fn new(sample_rate: f32) -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
            sample_rate,
            one_over_sample_rate: 1.0 / sample_rate,
        }
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.one_over_sample_rate = 1.0 / sample_rate;
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Get one over sample rate
    pub fn one_over_sample_rate(&self) -> f32 {
        self.one_over_sample_rate
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    /// Process one sample using Direct Form II transposed
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }
}

/// Biquad allpass filter for phaser effect
#[derive(Debug, Clone)]
pub struct BiquadAllpass {
    filter: BiquadFilter,
    radius: f32,
}

impl BiquadAllpass {
    /// Create a new biquad allpass filter
    pub fn new(sample_rate: f32) -> Self {
        Self {
            filter: BiquadFilter::new(sample_rate),
            radius: 1.32,
        }
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.filter.set_sample_rate(sample_rate);
    }

    /// Set the radius (controls bandwidth/resonance)
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    /// Set the center frequency
    pub fn set_frequency(&mut self, freq: f32) {
        // BiquadAllpass.cpp:30-37: H(z) = (1/r² - 2/r·cos·z⁻¹ + z⁻²) / (1 - 2/r·cos·z⁻¹ + 1/r²·z⁻²),
        // cosinus de juce évalué en float.
        let freq_rad = freq * self.filter.one_over_sample_rate() * 2.0 * PI;
        let b1 = -2.0 / self.radius * juce_cos_f32(freq_rad);
        let r2 = 1.0 / (self.radius * self.radius);
        self.filter.b0 = r2;
        self.filter.b1 = b1;
        self.filter.b2 = 1.0;
        self.filter.a1 = b1;
        self.filter.a2 = r2;
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

/// Résonateur du filtre de formants (BiquadResonator.h + BiquadFilter.cpp:26-37).
///
/// Coefficients et mémoires en double comme l'amont: le pôle de rayon 0,996 a une bande passante
/// de ~4e-3 rad, un coefficient `a1 ≈ -2` quantifié en f32 (ulp 1,2e-7) y fait ~1e-4 d'erreur
/// relative sur le gain de crête.
#[derive(Debug, Clone)]
pub struct BiquadResonator {
    b0: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
    radius: f32,
    freq: f32,
    one_over_sample_rate: f32,
}

/// Gain factor for resonator (RESONATOR_GAIN_FACTOR, double)
const RESONATOR_GAIN_FACTOR: f64 = 0.08;

impl BiquadResonator {
    /// Create a new biquad resonator
    pub fn new(sample_rate: f32) -> Self {
        let mut resonator = Self {
            // Zéros à 0 et Nyquist (BiquadResonator.h:19-26), b1 = 0.
            b0: RESONATOR_GAIN_FACTOR,
            b2: -RESONATOR_GAIN_FACTOR,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
            radius: 0.996,
            freq: 2000.0,
            one_over_sample_rate: 1.0 / sample_rate,
        };
        resonator.recalculate_coefficients();
        resonator
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.one_over_sample_rate = 1.0 / sample_rate;
        self.recalculate_coefficients();
    }

    /// Set the radius (controls bandwidth/Q)
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
        self.recalculate_coefficients();
    }

    /// Set the center frequency
    pub fn set_frequency(&mut self, freq: f32) {
        self.freq = freq;
        self.recalculate_coefficients();
    }

    /// Recalculate filter coefficients
    fn recalculate_coefficients(&mut self) {
        // BiquadResonator.h:52: l'argument est un double (`2 * M_PI * float * float`) et la Padé de
        // juce `FastMathApproximations::cos<double>` en est évaluée (un cosinus exact décalerait le
        // pôle de ~1e-4 relatif sur le gain de crête); `m_a2 = m_radius * m_radius` est un produit float.
        let w0 = 2.0 * core::f64::consts::PI * self.freq as f64 * self.one_over_sample_rate as f64;
        self.a1 = -2.0 * self.radius as f64 * juce_cos(w0);
        self.a2 = (self.radius * self.radius) as f64;
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    /// Une étape en double (BiquadFilter::doFilter, forme directe II): le filtre de formants
    /// enchaîne ses deux résonateurs sans arrondi intermédiaire en float.
    #[inline]
    pub fn process_f64(&mut self, input: f64) -> f64 {
        let left_side = input - self.z2 * self.a2 - self.z1 * self.a1;
        let output = self.b2 * self.z2 + self.b0 * left_side;
        self.z2 = self.z1;
        self.z1 = left_side;
        output
    }

    /// Process one sample
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        self.process_f64(input as f64) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_basic() {
        let mut filter = BiquadFilter::new(44100.0);

        // Process some samples
        for i in 0..1000 {
            let input = if i % 100 < 50 { 1.0 } else { -1.0 };
            let output = filter.process(input);
            assert!(output.is_finite(), "Filter produced invalid output");
        }
    }

    #[test]
    fn test_biquad_allpass() {
        let mut filter = BiquadAllpass::new(44100.0);
        filter.set_frequency(1000.0);

        // Process some samples
        for i in 0..1000 {
            let input = if i % 100 < 50 { 1.0 } else { -1.0 };
            let output = filter.process(input);
            assert!(output.is_finite(), "Allpass produced invalid output");
        }
    }

    #[test]
    fn test_biquad_resonator() {
        let mut filter = BiquadResonator::new(44100.0);
        filter.set_frequency(1000.0);

        // Process some samples
        for i in 0..1000 {
            let input = if i % 100 < 50 { 0.5 } else { -0.5 };
            let output = filter.process(input);
            assert!(output.is_finite(), "Resonator produced invalid output");
        }
    }
}
