//! Bruit blanc déterministe filtré LP puis HP, VAOnePole TPT amont.
use super::PresetOscillator;
use crate::preset::{OscillatorParams, OscillatorType};

#[derive(Clone)]
pub struct NoiseOscillator { runtime: PresetOscillator, params: OscillatorParams }
impl NoiseOscillator {
    pub fn new(sample_rate: f32) -> Self {
        let params = OscillatorParams { osc_type: OscillatorType::Noise, lp: 20000.0, hp: 20.0, ..Default::default() };
        let mut runtime = PresetOscillator::new(sample_rate); runtime.configure(&params);
        Self { runtime, params }
    }
    pub fn set_lp_freq(&mut self, frequency: f32) { self.params.lp = frequency; self.runtime.configure(&self.params); }
    pub fn set_hp_freq(&mut self, frequency: f32) { self.params.hp = frequency; self.runtime.configure(&self.params); }
    pub fn set_filter_freqs(&mut self, low: f32, high: f32) {
        self.params.lp = low; self.params.hp = high; self.runtime.configure(&self.params);
    }
    pub fn process(&mut self) -> f32 { self.runtime.process() }
    pub fn set_sample_rate(&mut self, sample_rate: f32) { self.runtime.set_sample_rate(sample_rate); }
    pub fn reset(&mut self) { self.runtime.reset(); }
}
