//! Oscillateur PM à deux opérateurs, cœur commun traduit de PMOscillator.cpp.
use super::{Oscillator, PresetOscillator};
use crate::preset::{OscillatorParams, OscillatorType};

#[derive(Clone)]
pub struct PmOscillator {
    runtime: PresetOscillator,
    params: OscillatorParams,
    carrier_table: usize,
    modulator_table: usize,
}
impl PmOscillator {
    pub fn new(sample_rate: f32) -> Self {
        let params = OscillatorParams { osc_type: OscillatorType::PM, ..Default::default() };
        let mut oscillator = Self { runtime: PresetOscillator::new(sample_rate), params, carrier_table: 0, modulator_table: 0 };
        oscillator.apply(); oscillator
    }
    fn apply(&mut self) {
        self.runtime.configure(&self.params);
        self.runtime.set_raw_tables([self.carrier_table; 4]);
        self.runtime.set_raw_modulator_table(self.modulator_table);
    }
    pub fn set_pm_amount(&mut self, amount: f32) { self.params.fm = amount; self.apply(); }
    pub fn pm_amount(&self) -> f32 { self.params.fm }
    pub fn set_carrier_ratio(&mut self, ratio: i32) { self.params.carrier_ratio = ratio.max(1); self.apply(); }
    pub fn set_modulator_ratio(&mut self, ratio: i32) { self.params.modulator_ratio = ratio.max(1); self.apply(); }
    pub fn set_ratios(&mut self, carrier: i32, modulator: i32) {
        self.params.carrier_ratio = carrier.max(1); self.params.modulator_ratio = modulator.max(1); self.apply();
    }
    pub fn carrier_ratio(&self) -> i32 { self.params.carrier_ratio }
    pub fn modulator_ratio(&self) -> i32 { self.params.modulator_ratio }
    pub fn set_carrier_wavetable(&mut self, index: usize) { self.carrier_table = index.min(159); self.apply(); }
    pub fn set_modulator_wavetable(&mut self, index: usize) { self.modulator_table = index.min(159); self.apply(); }
    pub fn set_wavetables(&mut self, carrier: usize, modulator: usize) {
        self.carrier_table = carrier.min(159); self.modulator_table = modulator.min(159); self.apply();
    }
    pub fn carrier_wavetable_name(&self) -> &'static str { crate::wavetables::WAVETABLE_NAMES[self.carrier_table] }
    pub fn modulator_wavetable_name(&self) -> &'static str { crate::wavetables::WAVETABLE_NAMES[self.modulator_table] }
}
impl Oscillator for PmOscillator {
    fn process(&mut self) -> f32 { self.runtime.process() }
    fn set_frequency(&mut self, frequency: f32) { self.runtime.set_frequency(frequency); }
    fn set_sample_rate(&mut self, sample_rate: f32) { self.runtime.set_sample_rate(sample_rate); }
    fn reset(&mut self) { self.runtime.reset(); }
    fn randomize_phase(&mut self) { self.runtime.randomize_phase(); }
}
