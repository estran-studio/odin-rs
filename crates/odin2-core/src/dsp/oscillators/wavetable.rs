//! Lecteur 1D bandlimité, WavetableOsc1D.cpp:43–93.
use super::Oscillator;
use super::preset_oscillator::{read_table, subtable_index, wrap};

#[derive(Clone)]
pub struct WavetableOscillator {
    phase: f64,
    increment: f64,
    frequency: f32,
    inverse_rate: f64,
    wavetable_index: usize,
    subtable_index: usize,
    random: u32,
}
impl WavetableOscillator {
    pub fn new(sample_rate: f32) -> Self {
        let mut oscillator = Self { phase: 0.0, increment: 0.0, frequency: 440.0,
            inverse_rate: 1.0 / sample_rate as f64, wavetable_index: 0,
            subtable_index: subtable_index(440.0), random: 0x12345678 };
        oscillator.set_frequency(440.0); oscillator
    }
    pub fn set_wavetable(&mut self, index: usize) { self.wavetable_index = index.min(159); }
    pub fn set_wavetable_by_name(&mut self, name: &str) -> bool {
        if let Some(index) = crate::wavetables::WAVETABLE_NAMES.iter().position(|candidate| *candidate == name) {
            self.set_wavetable(index); true
        } else { false }
    }
    pub fn wavetable_index(&self) -> usize { self.wavetable_index }
    pub fn wavetable_name(&self) -> &'static str { crate::wavetables::WAVETABLE_NAMES[self.wavetable_index] }
}
impl Oscillator for WavetableOscillator {
    fn process(&mut self) -> f32 {
        let output = read_table(crate::wavetables::get_subtable(self.wavetable_index, self.subtable_index), self.phase);
        self.phase = wrap(self.phase + self.increment); output
    }
    fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency.clamp(-20480.0, 20480.0);
        self.increment = self.frequency as f64 * self.inverse_rate;
        self.subtable_index = subtable_index(self.frequency);
    }
    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.inverse_rate = 1.0 / sample_rate as f64;
        self.increment = self.frequency as f64 * self.inverse_rate;
    }
    fn reset(&mut self) { self.phase = 0.0; }
    fn randomize_phase(&mut self) {
        let mut x = self.random; x ^= x << 13; x ^= x >> 17; x ^= x << 5;
        self.random = x; self.phase = x as f64 / (u32::MAX as f64 + 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_frequency_reverses_phase_without_dc_fallback() {
        let mut positive = WavetableOscillator::new(48000.0);
        let mut negative = WavetableOscillator::new(48000.0);
        positive.set_frequency(375.0); negative.set_frequency(-375.0);
        for _ in 0..512 {
            assert!((positive.process() + negative.process()).abs() < 2e-5);
        }
    }
    #[test]
    fn subtable_boundary_uses_next_band_at_threshold() {
        assert_eq!(subtable_index(27.499), 0);
        assert_eq!(subtable_index(27.5), 1);
        assert_eq!(subtable_index(-27.5), 1);
    }
}
