//! LFO Odin 2: tables originales et modulation exponentielle de 48 demi-tons.
//! Sources: LFO.cpp:18–73, LFO.h:38–76, LFOTableData.cpp.
use alloc::rc::Rc;
use super::{DrawTableBank, Oscillator};
use super::preset_oscillator::{pitch, read_table, wrap};
pub use crate::preset::LfoWaveform;
use crate::preset::{LfoParams, sync_ratio};

#[derive(Clone)]
pub struct Lfo {
    phase: f64,
    frequency: f32,
    inverse_rate: f64,
    waveform: LfoWaveform,
    modulation: f32,
    tempo: f32,
    sync: bool,
    sync_ratio: f32,
    reset_active: bool,
    sh_value: f32,
    random: u32,
    draws: Option<Rc<DrawTableBank>>,
}
impl Lfo {
    pub fn new(sample_rate: f32) -> Self {
        Self { phase: 0.0, frequency: 1.0, inverse_rate: 1.0 / sample_rate as f64,
            waveform: LfoWaveform::Sine, modulation: 0.0, tempo: 120.0,
            sync: false, sync_ratio: 3.0 / 16.0, reset_active: false,
            sh_value: 0.137, random: 12345, draws: None }
    }
    pub fn configure(&mut self, params: &LfoParams) {
        self.waveform = params.waveform;
        self.frequency = params.frequency;
        self.sync = params.sync;
        self.sync_ratio = sync_ratio(params.sync_numerator, params.sync_denominator);
        self.reset_active = params.reset;
    }
    pub fn set_draw_tables(&mut self, tables: Rc<DrawTableBank>) { self.draws = Some(tables); }
    pub fn set_modulation(&mut self, modulation: f32) { self.modulation = modulation; }
    pub fn set_tempo(&mut self, bpm: f32) { self.tempo = bpm; }
    pub fn retrigger(&mut self) { if self.reset_active { self.reset(); } }
    pub fn set_frequency(&mut self, frequency: f32) { self.frequency = frequency; }
    pub fn set_freq_from_bpm(&mut self, bpm: f32) {
        self.tempo = bpm; self.frequency = bpm / self.sync_ratio / 240.0;
    }
    /// Ratio physique (et non les indices sérialisés des menus).
    pub fn set_sync_ratio(&mut self, numerator: f32, denominator: f32) {
        self.sync_ratio = numerator / denominator;
    }
    pub fn set_waveform(&mut self, waveform: LfoWaveform) { self.waveform = waveform; }
    pub fn waveform(&self) -> LfoWaveform { self.waveform }
    fn random_bipolar(&mut self) -> f32 {
        let mut x = self.random; x ^= x << 13; x ^= x >> 17; x ^= x << 5;
        self.random = x; x as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
    pub fn process(&mut self) -> f32 {
        let frequency = if self.sync { self.tempo / self.sync_ratio / 240.0 } else { self.frequency };
        // LFO::update (LFO.h:38–54): fréquence de base float promue en double, plafonnée à OSC_FO_MAX.
        let frequency = (frequency as f64 * pitch(self.modulation * 48.0) as f64).min(20480.0);
        let next = self.phase + frequency * self.inverse_rate;
        if self.waveform == LfoWaveform::SampleAndHold {
            if !(0.0..1.0).contains(&next) { self.sh_value = self.random_bipolar(); }
            self.phase = wrap(next);
            return self.sh_value;
        }
        let value = match self.waveform {
            LfoWaveform::Sine => read_table(crate::wavetables::get_subtable(0, 0), self.phase),
            LfoWaveform::Saw => read_table(crate::wavetables::get_subtable(5, 0), self.phase),
            LfoWaveform::Triangle => read_table(crate::wavetables::get_subtable(6, 0), self.phase),
            LfoWaveform::Square => self.read_lfo(9),
            LfoWaveform::Square25 => self.read_lfo(10),
            LfoWaveform::Square12 => self.read_lfo(11),
            LfoWaveform::Spike => self.read_lfo(12),
            LfoWaveform::Pyramid4 => self.read_lfo(0),
            LfoWaveform::Pyramid6 => self.read_lfo(1),
            LfoWaveform::Pyramid8 => self.read_lfo(2),
            LfoWaveform::Pyramid12 => self.read_lfo(3),
            LfoWaveform::Stair3 => self.read_lfo(4),
            LfoWaveform::Stair4 => self.read_lfo(5),
            LfoWaveform::Stair6 => self.read_lfo(6),
            LfoWaveform::Stair8 => self.read_lfo(7),
            LfoWaveform::Stair12 => self.read_lfo(8),
            LfoWaveform::Wavedraw1 | LfoWaveform::Wavedraw2 | LfoWaveform::Wavedraw3 |
            LfoWaveform::Chipdraw1 | LfoWaveform::Chipdraw2 | LfoWaveform::Chipdraw3 => {
                let draw = self.waveform as usize - LfoWaveform::Wavedraw1 as usize;
                read_table(self.draws.as_ref().expect("LFO dessin: installer DrawTableBank avant rendu").subtable(draw, 0), self.phase)
            }
            LfoWaveform::SampleAndHold => unreachable!(),
        };
        self.phase = wrap(next); value
    }
    fn read_lfo(&self, index: usize) -> f32 {
        read_table(&crate::wavetables::lfo_data::LFO_DATA[index], self.phase)
    }
    pub fn set_sample_rate(&mut self, sample_rate: f32) { self.inverse_rate = 1.0 / sample_rate as f64; }
    pub fn reset(&mut self) { self.phase = 0.0; }
    pub fn randomize_phase(&mut self) { self.phase = self.random_bipolar() as f64 * 0.5 + 0.5; }
}
impl Oscillator for Lfo {
    fn process(&mut self) -> f32 { Lfo::process(self) }
    fn set_frequency(&mut self, frequency: f32) { Lfo::set_frequency(self, frequency); }
    fn set_sample_rate(&mut self, sample_rate: f32) { Lfo::set_sample_rate(self, sample_rate); }
    fn reset(&mut self) { Lfo::reset(self); }
    fn randomize_phase(&mut self) { Lfo::randomize_phase(self); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tempo_sync_and_modulation() {
        let mut lfo = Lfo::new(48000.0);
        let params = LfoParams { sync: true, sync_numerator: 0, sync_denominator: 2, ..Default::default() };
        lfo.configure(&params); lfo.set_tempo(120.0); lfo.set_modulation(0.25);
        for _ in 0..6000 { lfo.process(); }
        assert!((lfo.phase - 0.5).abs() < 1e-3);
    }
    #[test]
    fn sample_hold_stays_constant_between_wraps() {
        let mut lfo = Lfo::new(48000.0);
        lfo.set_waveform(LfoWaveform::SampleAndHold); lfo.set_frequency(10.0);
        let initial = lfo.process();
        for _ in 0..4000 { assert_eq!(initial, lfo.process()); }
    }
}
