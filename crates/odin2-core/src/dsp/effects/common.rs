//! Primitives internes traduites des filtres Odin 2 (GPL-3.0-or-later).

extern crate alloc;
use alloc::{boxed::Box, vec};

pub(super) const MAX_SAMPLE_RATE: f32 = 192000.0;
pub(super) fn buffer(size: usize) -> Box<[f32]> { vec![0.0; size].into_boxed_slice() }
pub(super) fn interpolate(buffer: &[f32], index: f32, floor: bool) -> f32 {
    let low = if floor { libm::floorf(index) as i32 } else { index as i32 };
    let fraction = index - low as f32;
    let n = buffer.len() as i32;
    let a = low.rem_euclid(n) as usize;
    let b = (low + 1).rem_euclid(n) as usize;
    (1.0 - fraction) * buffer[a] + fraction * buffer[b]
}

/// Même approximation Padé que JUCE 6 FastMathApproximations::cos.
/// https://github.com/juce-framework/JUCE/blob/6.0.8/modules/juce_dsp/maths/juce_FastMathApproximations.h
fn fast_cos(x: f32) -> f32 {
    let x2 = x * x;
    -(-39251520.0 + x2 * (18471600.0 + x2 * (-1075032.0 + 14615.0 * x2)))
        / (39251520.0 + x2 * (1154160.0 + x2 * (16632.0 + x2 * 127.0)))
}
fn fast_exp(x: f64) -> f64 {
    (1680.0 + x * (840.0 + x * (180.0 + x * (20.0 + x))))
        / (1680.0 + x * (-840.0 + x * (180.0 + x * (-20.0 + x))))
}

/// BiquadFilter.cpp:32-44, BiquadAllpass.cpp:27-38.
#[derive(Clone, Copy)]
pub(super) struct Allpass {
    sample_rate: f32,
    radius: f32,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}
impl Allpass {
    pub fn new(sample_rate: f32, radius: f32) -> Self {
        Self { sample_rate, radius, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 }
    }
    pub fn set_sample_rate(&mut self, rate: f32) { self.sample_rate = rate; }
    pub fn set_radius(&mut self, radius: f32) { self.radius = radius; }
    pub fn set_frequency(&mut self, frequency: f32) {
        let radians = ((frequency * (1.0 / self.sample_rate) * 2.0) as f64 * core::f64::consts::PI) as f32;
        self.a1 = (-2.0 / self.radius * fast_cos(radians)) as f64;
        self.a2 = (1.0 / (self.radius * self.radius)) as f64;
    }
    pub fn process(&mut self, input: f64) -> f64 {
        let left = input - self.z2 * self.a2 - self.z1 * self.a1;
        let output = self.z2 + self.a1 * self.z1 + self.a2 * left;
        self.z2 = self.z1;
        self.z1 = left;
        output
    }
    pub fn reset(&mut self) { self.z1 = 0.0; self.z2 = 0.0; }
}

/// DCBlockingFilter.h:21-43.
pub(super) struct DcBlock {
    previous_input: f64,
    previous_output: f64,
    radius: f64,
}
impl DcBlock {
    pub fn new(sample_rate: f32) -> Self {
        let mut s = Self { previous_input: 0.0, previous_output: 0.0, radius: 0.995 };
        s.set_sample_rate(sample_rate);
        s
    }
    pub fn set_sample_rate(&mut self, rate: f32) {
        self.radius = if rate > 120000.0 { 0.997 } else if rate > 90000.0 { 0.9965 } else { 0.995 };
    }
    pub fn process(&mut self, input: f32) -> f32 {
        let y = input as f64 - self.previous_input + self.radius * self.previous_output;
        self.previous_input = input as f64;
        self.previous_output = y;
        y as f32
    }
    pub fn reset(&mut self) { self.previous_input = 0.0; self.previous_output = 0.0; }
}

/// VAOnePoleFilter.cpp:45-72, OdinFilterBase.h:35-63.
pub(super) struct Highpass {
    sample_rate: f32,
    frequency: f32,
    modulation: f32,
    alpha: f64,
    z: f64,
    dirty: bool,
}
impl Highpass {
    pub fn new(sample_rate: f32) -> Self {
        Self { sample_rate, frequency: 80.0, modulation: 0.0, alpha: 1.0, z: 0.0, dirty: true }
    }
    pub fn set_sample_rate(&mut self, rate: f32) { self.sample_rate = rate; self.dirty = true; }
    pub fn set_frequency(&mut self, frequency: f32) {
        if self.frequency != frequency { self.frequency = frequency; self.dirty = true; }
    }
    pub fn set_modulation(&mut self, value: f32) {
        if self.modulation != value { self.modulation = value; self.dirty = true; }
    }
    pub fn process(&mut self, input: f32) -> f32 {
        if self.dirty {
            let mut frequency = self.frequency;
            if self.modulation != 0.0 {
                let semitones = self.modulation * 64.0;
                let multiplier = if semitones > -48.0 && semitones < 48.0 {
                    fast_exp(0.05776226504 * semitones as f64) as f32
                } else { libm::exp(0.05776226504 * semitones as f64) as f32 };
                frequency *= multiplier;
            }
            frequency = frequency.clamp(20.0, 20000.0);
            // C++ calcule T = 1 / m_samplerate en float, puis le reste en double.
            let period = (1.0 / self.sample_rate) as f64;
            let warped = (2.0 / period) * libm::tan(2.0 * core::f64::consts::PI * frequency as f64 * period / 2.0);
            let g = warped * period / 2.0;
            self.alpha = g / (1.0 + g);
            self.dirty = false;
        }
        let v = (input as f64 - self.z) * self.alpha;
        let low = v + self.z;
        self.z = v + low;
        (input as f64 - low) as f32
    }
    pub fn reset(&mut self) { self.z = 0.0; }
}
