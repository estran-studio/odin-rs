//! Fast math approximations for DSP
//!
//! These functions provide fast approximations of common math operations
//! used in audio DSP. They trade some accuracy for speed.
//!
//! Based on JUCE's FastMathApproximations and standard DSP techniques.

use libm::{expf, powf, sinf};

/// ln(2) / 12 - used for semitone to frequency conversion
pub const LN2_OVER_12: f32 = 0.057762265;

/// Approximation de Padé utilisée par `juce::dsp::FastMathApproximations::exp`.
/// Les consommateurs doivent respecter le domaine de leur algorithme C++.
#[inline]
pub fn fast_exp(x: f32) -> f32 {
    let numerator = 1680.0 + x * (840.0 + x * (180.0 + x * (20.0 + x)));
    let denominator = 1680.0 + x * (-840.0 + x * (180.0 + x * (-20.0 + x)));
    numerator / denominator
}

/// Fast tanh approximation
///
/// Uses rational function approximation. Accurate within ~0.01% for inputs in [-3, 3]
/// Essential for filter saturation and soft clipping.
#[inline]
pub fn fast_tanh(x: f32) -> f32 {
    let x2 = x * x;
    let a = x * (135135.0 + x2 * (17325.0 + x2 * (378.0 + x2)));
    let b = 135135.0 + x2 * (62370.0 + x2 * (3150.0 + x2 * 28.0));
    a / b
}

/// Fast tangent approximation
///
/// Used for frequency prewarping in bilinear transform filters.
/// Input should be in range (-PI/2, PI/2).
#[inline]
pub fn fast_tan(x: f32) -> f32 {
    let x2 = x * x;
    let numerator = x * (-135135.0 + x2 * (17325.0 + x2 * (-378.0 + x2)));
    let denominator = -135135.0 + x2 * (62370.0 + x2 * (-3150.0 + 28.0 * x2));
    numerator / denominator
}

/// Fast sine approximation using Bhaskara I's formula
///
/// Accurate within ~0.2% across the full range.
#[inline]
pub fn fast_sin(x: f32) -> f32 {
    // Normalize to [0, 2*PI]
    let x = x % core::f32::consts::TAU;
    let x = if x < 0.0 { x + core::f32::consts::TAU } else { x };

    // Use standard libm for now - can optimize later
    sinf(x)
}

/// Fast cosine approximation
#[inline]
pub fn fast_cos(x: f32) -> f32 {
    fast_sin(x + core::f32::consts::FRAC_PI_2)
}

/// Convert semitones to frequency multiplier
///
/// Uses the formula: multiplier = 2^(semitones/12) = e^(ln(2)/12 * semitones)
/// This is the core function for pitch modulation.
#[inline]
pub fn pitch_shift_multiplier(semitones: f32) -> f32 {
    // Use fast exp for typical range (-48 to +48 semitones)
    if semitones > -48.0 && semitones < 48.0 {
        fast_exp(LN2_OVER_12 * semitones)
    } else {
        expf(LN2_OVER_12 * semitones)
    }
}

/// Convert MIDI note number to frequency (Hz)
///
/// A4 (note 69) = 440 Hz
#[inline]
pub fn midi_to_freq(note: u8) -> f32 {
    // Tunings::Tuning::frequencyForMidiNote (TuningsImpl.h:495): MIDI_0_FREQ * 2^(note/12) en double;
    // l'approximation de Padé de `pitch_shift_multiplier` s'écarte de 3e-5 à ±36 demi-tons.
    (8.17579891564371 * libm::exp2(note as f64 / 12.0)) as f32
}

/// Convert frequency to MIDI note number
#[inline]
pub fn freq_to_midi(freq: f32) -> f32 {
    69.0 + 12.0 * libm::log2f(freq / 440.0)
}

/// Linear interpolation
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Clamp value to range [min, max]
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Soft clipping using tanh
#[inline]
pub fn soft_clip(x: f32, drive: f32) -> f32 {
    fast_tanh(x * drive) / fast_tanh(drive)
}

/// Calculate glide coefficient
///
/// Returns a smoothing factor for portamento.
/// glide_input should be normalized [0, 1]
#[inline]
pub fn calculate_glide(glide_input: f32) -> f32 {
    if glide_input < 0.01 {
        0.0
    } else {
        0.9985 + glide_input * 0.0014
    }
}

/// Decibels to linear amplitude
#[inline]
pub fn db_to_linear(db: f32) -> f32 {
    powf(10.0, db / 20.0)
}

/// Linear amplitude to decibels
#[inline]
pub fn linear_to_db(linear: f32) -> f32 {
    20.0 * libm::log10f(linear.max(1e-10))
}

/// Fast inverse square root (Quake III style, for reference)
#[inline]
pub fn fast_inv_sqrt(x: f32) -> f32 {
    let half_x = 0.5 * x;
    let i = x.to_bits();
    let i = 0x5f3759df - (i >> 1);
    let y = f32::from_bits(i);
    y * (1.5 - half_x * y * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_shift_multiplier() {
        // Octave up should double frequency
        let octave_up = pitch_shift_multiplier(12.0);
        assert!((octave_up - 2.0).abs() < 0.01);

        // Octave down should halve frequency
        let octave_down = pitch_shift_multiplier(-12.0);
        assert!((octave_down - 0.5).abs() < 0.01);

        // No shift
        let no_shift = pitch_shift_multiplier(0.0);
        assert!((no_shift - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_midi_to_freq() {
        // A4 should be 440 Hz
        let a4 = midi_to_freq(69);
        assert!((a4 - 440.0).abs() < 0.1);

        // A5 should be 880 Hz
        let a5 = midi_to_freq(81);
        assert!((a5 - 880.0).abs() < 0.5);

        // Middle C (C4) should be ~261.6 Hz
        let c4 = midi_to_freq(60);
        assert!((c4 - 261.63).abs() < 1.0);
    }

    #[test]
    fn test_fast_tanh() {
        // tanh(0) = 0
        assert!(fast_tanh(0.0).abs() < 0.001);

        // tanh should saturate to +/- 1
        assert!((fast_tanh(5.0) - 1.0).abs() < 0.01);
        assert!((fast_tanh(-5.0) + 1.0).abs() < 0.01);
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 0.001);
        assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < 0.001);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < 0.001);
    }
}
