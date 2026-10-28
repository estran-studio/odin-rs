//! Formant Filter
//!
//! A filter that simulates vowel sounds by using two resonators
//! tuned to formant frequencies. Supports smooth transitions between vowels.

use super::biquad::BiquadResonator;
use super::Filter;
use crate::dsp::math::db_to_linear;

/// Output scalar for formant filter
const FORMANT_OUTPUT_SCALAR: f32 = 0.35;

/// dB compensation at 88kHz sample rate
const FORMANT_DB_AT_88KHZ: f32 = -11.0;

/// dB compensation at 192kHz sample rate
const FORMANT_DB_AT_192KHZ: f32 = -18.0;

/// Vowel types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Vowel {
    A = 0,
    E = 1,
    I = 2,
    O = 3,
    U = 4,
    Ae = 5, // Ä
    Oe = 6, // Ö
    Ue = 7, // Ü
}

impl From<u8> for Vowel {
    fn from(v: u8) -> Self {
        match v {
            0 => Vowel::A,
            1 => Vowel::E,
            2 => Vowel::I,
            3 => Vowel::O,
            4 => Vowel::U,
            5 => Vowel::Ae,
            6 => Vowel::Oe,
            7 => Vowel::Ue,
            _ => Vowel::A,
        }
    }
}

/// Formant frequencies for each vowel [formant1, formant2]
const FORMANT_TABLE: [[f32; 2]; 8] = [
    [1000.0, 1400.0], // A
    [500.0, 2300.0],  // E
    [320.0, 3200.0],  // I
    [500.0, 1000.0],  // O
    [320.0, 800.0],   // U
    [700.0, 1800.0],  // Ä
    [500.0, 1500.0],  // Ö
    [320.0, 1650.0],  // Ü
];

/// Formant filter
///
/// Uses two resonators to create vowel-like sounds with smooth
/// transition between two selected vowels.
pub struct FormantFilter {
    /// First resonator
    resonator1: BiquadResonator,
    /// Second resonator
    resonator2: BiquadResonator,

    /// Sample rate
    sample_rate: f32,

    /// Left vowel
    vowel_left: usize,
    /// Right vowel
    vowel_right: usize,

    /// Transition (0.0 = left vowel, 1.0 = right vowel)
    transition: f32,

    /// Parabola coefficients for resonator 1
    a0: f32,
    b0: f32,
    c0: f32,

    /// Parabola coefficients for resonator 2
    a1: f32,
    b1: f32,
    c1: f32,

    /// Sample rate gain compensation
    samplerate_gain_compensation: f32,
}

impl FormantFilter {
    /// Create a new formant filter
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            resonator1: BiquadResonator::new(sample_rate),
            resonator2: BiquadResonator::new(sample_rate),
            sample_rate,
            vowel_left: 0,
            vowel_right: 1,
            transition: 0.0,
            a0: 0.0,
            b0: 0.0,
            c0: 0.0,
            a1: 0.0,
            b1: 0.0,
            c1: 0.0,
            samplerate_gain_compensation: 1.0,
        };

        filter.update_parabolas();
        filter.update_samplerate_compensation(sample_rate);
        filter
    }

    /// Set the left vowel
    pub fn set_vowel_left(&mut self, vowel: Vowel) {
        self.vowel_left = vowel as usize;
        self.update_parabolas();
    }

    /// Set the right vowel
    pub fn set_vowel_right(&mut self, vowel: Vowel) {
        self.vowel_right = vowel as usize;
        self.update_parabolas();
    }

    /// Set transition between vowels (0.0 to 1.0)
    pub fn set_transition(&mut self, transition: f32) {
        self.transition = transition.clamp(0.0, 1.0);
    }

    /// Update parabola coefficients for smooth interpolation
    fn update_parabolas(&mut self) {
        // First formant
        let f0 = FORMANT_TABLE[self.vowel_left][0];
        let f1 = FORMANT_TABLE[self.vowel_right][0];
        let f2 = f0 * libm::powf(f1 / f0, 0.5);

        self.a0 = 2.0 * f1 - 4.0 * f2 + 2.0 * f0;
        self.b0 = 4.0 * f2 - 3.0 * f0 - f1;
        self.c0 = f0;

        // Second formant
        let f0 = FORMANT_TABLE[self.vowel_left][1];
        let f1 = FORMANT_TABLE[self.vowel_right][1];
        let f2 = f0 * libm::powf(f1 / f0, 0.5);

        self.a1 = 2.0 * f1 - 4.0 * f2 + 2.0 * f0;
        self.b1 = 4.0 * f2 - 3.0 * f0 - f1;
        self.c1 = f0;
    }

    /// Update sample rate compensation
    fn update_samplerate_compensation(&mut self, sample_rate: f32) {
        // Higher sample rates make the filter louder, so we compensate
        let (m, b) = if sample_rate < 88200.0 {
            (FORMANT_DB_AT_88KHZ / 44100.0, -44100.0 * (FORMANT_DB_AT_88KHZ / 44100.0))
        } else {
            let m = (FORMANT_DB_AT_192KHZ - FORMANT_DB_AT_88KHZ) / (192600.0 - 88200.0);
            let b = FORMANT_DB_AT_88KHZ - 88200.0 * m;
            (m, b)
        };

        let samplerate_db_compensation = m * sample_rate + b;
        self.samplerate_gain_compensation = db_to_linear(samplerate_db_compensation);
    }

    /// Update resonator frequencies based on transition
    fn update(&mut self) {
        let t = self.transition;
        let t2 = t * t;

        let freq1 = self.a0 * t2 + self.b0 * t + self.c0;
        let freq2 = self.a1 * t2 + self.b1 * t + self.c1;

        self.resonator1.set_frequency(freq1);
        self.resonator2.set_frequency(freq2);
    }
}

impl Filter for FormantFilter {
    fn process(&mut self, input: f32) -> f32 {
        self.update();

        // FormantFilter.cpp:62-64: chaîne en double, puis `* float * 0.35f`.
        let out = self.resonator1.process_f64(self.resonator2.process_f64(input as f64));

        (out * self.samplerate_gain_compensation as f64 * FORMANT_OUTPUT_SCALAR as f64) as f32
    }

    fn set_cutoff(&mut self, _cutoff: f32) {
        // Formant filter doesn't use cutoff in the traditional sense
        // The transition parameter controls the formant frequencies
    }

    fn set_resonance(&mut self, _resonance: f32) {
        // Resonance is fixed for formant filter
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.resonator1.set_sample_rate(sample_rate);
        self.resonator2.set_sample_rate(sample_rate);
        self.update_samplerate_compensation(sample_rate);
    }

    fn reset(&mut self) {
        self.resonator1.reset();
        self.resonator2.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formant_basic() {
        let mut filter = FormantFilter::new(44100.0);
        filter.set_vowel_left(Vowel::A);
        filter.set_vowel_right(Vowel::E);

        for i in 0..1000 {
            let input = if i % 100 < 50 { 0.5 } else { -0.5 };
            let output = filter.process(input);
            assert!(output.is_finite(), "Formant filter produced invalid output");
        }
    }

    #[test]
    fn test_formant_transition() {
        let mut filter = FormantFilter::new(44100.0);
        filter.set_vowel_left(Vowel::A);
        filter.set_vowel_right(Vowel::O);

        // Process at different transition values
        for transition in [0.0, 0.25, 0.5, 0.75, 1.0] {
            filter.set_transition(transition);
            filter.reset();

            for i in 0..100 {
                let input = if i % 20 < 10 { 0.5 } else { -0.5 };
                let output = filter.process(input);
                assert!(
                    output.is_finite(),
                    "Transition {} produced invalid output",
                    transition
                );
            }
        }
    }

    #[test]
    fn test_formant_all_vowels() {
        let vowels = [
            Vowel::A,
            Vowel::E,
            Vowel::I,
            Vowel::O,
            Vowel::U,
            Vowel::Ae,
            Vowel::Oe,
            Vowel::Ue,
        ];

        for &left in &vowels {
            for &right in &vowels {
                let mut filter = FormantFilter::new(44100.0);
                filter.set_vowel_left(left);
                filter.set_vowel_right(right);
                filter.set_transition(0.5);

                for i in 0..50 {
                    let input = if i % 10 < 5 { 0.5 } else { -0.5 };
                    let output = filter.process(input);
                    assert!(output.is_finite());
                }
            }
        }
    }
}
