//! Chiptune Odin 2: tables, arp trois pas et bruit quantifié suréchantillonné.
use super::PresetOscillator;
use crate::preset::{OscillatorParams, OscillatorType};

#[derive(Clone)]
pub struct ChiptuneOscillator {
    runtime: PresetOscillator,
    params: OscillatorParams,
    table: usize,
    volume: f32,
}
impl ChiptuneOscillator {
    pub fn new(sample_rate: f32) -> Self {
        let params = OscillatorParams { osc_type: OscillatorType::Chiptune, ..Default::default() };
        let mut oscillator = Self { runtime: PresetOscillator::new(sample_rate), params, table: 2, volume: 1.0 };
        oscillator.apply(); oscillator
    }
    fn apply(&mut self) { self.runtime.configure(&self.params); self.runtime.set_raw_tables([self.table; 4]); }
    pub fn set_sample_rate(&mut self, sample_rate: f32) { self.runtime.set_sample_rate(sample_rate); }
    pub fn set_frequency(&mut self, frequency: f32) { self.runtime.set_frequency(frequency); }
    pub fn select_wavetable(&mut self, index: usize) { self.table = index.min(159); self.apply(); }
    pub fn set_arp_enabled(&mut self, enabled: bool) { self.params.arp_on = enabled; self.apply(); }
    pub fn set_arp_speed(&mut self, speed: f32) { self.params.arp_speed = speed; self.apply(); }
    pub fn set_arp_semitone(&mut self, step: usize, semitones: i32) {
        match step { 0 => self.params.step_1 = semitones, 1 => self.params.step_2 = semitones,
            2 => self.params.step_3 = semitones, _ => return }
        self.apply();
    }
    pub fn set_arp_step_three(&mut self, active: bool) { self.params.step_3_on = active; self.apply(); }
    pub fn set_noise_enabled(&mut self, enabled: bool) {
        if enabled != self.params.chipnoise { self.runtime.reset(); }
        self.params.chipnoise = enabled; self.apply();
    }
    pub fn set_volume(&mut self, volume: f32) { self.volume = volume.clamp(0.0, 1.0); }
    pub fn reset(&mut self) { self.runtime.reset(); }
    pub fn process(&mut self) -> f32 { self.runtime.process() * self.volume }
}
