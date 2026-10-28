//! Oscillateur vectoriel: quatre tables interpolées selon la géométrie amont 1–2/0–3.
use super::PresetOscillator;
use crate::preset::{OscillatorParams, OscillatorType};

#[derive(Clone)]
pub struct VectorOscillator {
    runtime: PresetOscillator,
    params: OscillatorParams,
    tables: [usize; 4],
}
impl VectorOscillator {
    pub fn new(sample_rate: f32) -> Self {
        let params = OscillatorParams { osc_type: OscillatorType::Vector, ..Default::default() };
        let mut oscillator = Self { runtime: PresetOscillator::new(sample_rate), params, tables: [5, 2, 6, 0] };
        oscillator.apply(); oscillator
    }
    fn apply(&mut self) { self.runtime.configure(&self.params); self.runtime.set_raw_tables(self.tables); }
    pub fn set_sample_rate(&mut self, rate: f32) { self.runtime.set_sample_rate(rate); }
    pub fn set_frequency(&mut self, frequency: f32) { self.runtime.set_frequency(frequency); }
    pub fn set_x(&mut self, x: f32) { self.params.vec_x = x.clamp(0.0, 1.0); self.apply(); }
    pub fn set_y(&mut self, y: f32) { self.params.vec_y = y.clamp(0.0, 1.0); self.apply(); }
    pub fn set_xy(&mut self, x: f32, y: f32) { self.set_x(x); self.set_y(y); }
    pub fn select_wavetable(&mut self, corner: usize, wavetable_index: usize) {
        if corner < 4 && wavetable_index < 160 { self.tables[corner] = wavetable_index; self.apply(); }
    }
    pub fn reset(&mut self) { self.runtime.reset(); }
    pub fn randomize_phase(&mut self) { self.runtime.randomize_phase(); }
    pub fn process(&mut self) -> f32 { self.runtime.process() }
}
