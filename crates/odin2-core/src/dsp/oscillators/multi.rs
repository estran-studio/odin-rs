//! MultiOscillator amont: quatre lecteurs, désaccord quadratique et dispersion de position.
use super::PresetOscillator;
use crate::preset::{OscillatorParams, OscillatorType};

#[derive(Clone)]
pub struct MultiOscillator {
    runtime: PresetOscillator,
    params: OscillatorParams,
    table: usize,
    stereo_width: f32,
}
impl MultiOscillator {
    pub fn new(sample_rate: f32) -> Self {
        let params = OscillatorParams { osc_type: OscillatorType::Multi, ..Default::default() };
        let mut oscillator = Self { runtime: PresetOscillator::new(sample_rate), params, table: 7, stereo_width: 0.0 };
        oscillator.apply(); oscillator
    }
    fn apply(&mut self) { self.runtime.configure(&self.params); self.runtime.set_raw_tables([self.table; 4]); }
    pub fn set_sample_rate(&mut self, sample_rate: f32) { self.runtime.set_sample_rate(sample_rate); }
    pub fn set_frequency(&mut self, frequency: f32) { self.runtime.set_frequency(frequency); }
    /// Table 1D pour l'API élémentaire; PresetOscillator utilise les banques morph amont.
    pub fn set_wavetable(&mut self, index: usize) { self.table = index.min(159); self.apply(); }
    pub fn set_detune(&mut self, detune: f32) { self.params.detune = detune; self.apply(); }
    pub fn set_stereo_width(&mut self, width: f32) { self.stereo_width = width.clamp(0.0, 1.0); }
    pub fn reset(&mut self) { self.runtime.reset(); }
    pub fn randomize_phase(&mut self) { self.runtime.randomize_phase(); }
    pub fn process(&mut self) -> f32 { self.runtime.process() }
    pub fn process_stereo(&mut self) -> (f32, f32) {
        self.runtime.process();
        let values = self.runtime.multi_outputs();
        let mut left = 0.0; let mut right = 0.0;
        for (value, pan) in values.into_iter().zip([-1.0f32, -0.33, 0.33, 1.0]) {
            let pan = pan * self.stereo_width;
            left += value * (1.0 - pan) * 0.25;
            right += value * (1.0 + pan) * 0.25;
        }
        (left, right)
    }
}
