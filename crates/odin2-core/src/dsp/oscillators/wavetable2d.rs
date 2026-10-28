//! Morphing quatre tables: indices originaux de WavetableOsc2D.cpp:25–201.
use super::{Oscillator, OscillatorModulation, PresetOscillator};
use super::preset_oscillator::MORPH_TABLES;
use crate::preset::{OscillatorParams, OscillatorType};
pub const NUMBER_OF_WAVETABLES_2D: usize = 35;
pub const TABLES_PER_2D_WT: usize = 4;
#[derive(Clone, Copy)]
pub struct Wavetable2DPreset { pub name: &'static str, pub tables: [usize; 4] }
pub const WAVETABLE_2D_PRESETS: [Wavetable2DPreset; 35] = [
    Wavetable2DPreset { name: "Basic", tables: MORPH_TABLES[0] },
    Wavetable2DPreset { name: "Birds", tables: MORPH_TABLES[1] },
    Wavetable2DPreset { name: "BagPipe", tables: MORPH_TABLES[2] },
    Wavetable2DPreset { name: "Glass", tables: MORPH_TABLES[3] },
    Wavetable2DPreset { name: "FMSynth", tables: MORPH_TABLES[4] },
    Wavetable2DPreset { name: "BrokenSine", tables: MORPH_TABLES[5] },
    Wavetable2DPreset { name: "Skyline", tables: MORPH_TABLES[6] },
    Wavetable2DPreset { name: "Perlin", tables: MORPH_TABLES[7] },
    Wavetable2DPreset { name: "Rectangular", tables: MORPH_TABLES[8] },
    Wavetable2DPreset { name: "Bitreduced", tables: MORPH_TABLES[9] },
    Wavetable2DPreset { name: "Strings", tables: MORPH_TABLES[10] },
    Wavetable2DPreset { name: "Piano", tables: MORPH_TABLES[11] },
    Wavetable2DPreset { name: "Organ", tables: MORPH_TABLES[12] },
    Wavetable2DPreset { name: "Oboe", tables: MORPH_TABLES[13] },
    Wavetable2DPreset { name: "Trumpet", tables: MORPH_TABLES[14] },
    Wavetable2DPreset { name: "Legacy1", tables: MORPH_TABLES[15] },
    Wavetable2DPreset { name: "Legacy2", tables: MORPH_TABLES[16] },
    Wavetable2DPreset { name: "Legacy3", tables: MORPH_TABLES[17] },
    Wavetable2DPreset { name: "Legacy4", tables: MORPH_TABLES[18] },
    Wavetable2DPreset { name: "Voice1", tables: MORPH_TABLES[19] },
    Wavetable2DPreset { name: "Voice2", tables: MORPH_TABLES[20] },
    Wavetable2DPreset { name: "Voice3", tables: MORPH_TABLES[21] },
    Wavetable2DPreset { name: "Voice4", tables: MORPH_TABLES[22] },
    Wavetable2DPreset { name: "Additive1", tables: MORPH_TABLES[23] },
    Wavetable2DPreset { name: "Additive2", tables: MORPH_TABLES[24] },
    Wavetable2DPreset { name: "Additive3", tables: MORPH_TABLES[25] },
    Wavetable2DPreset { name: "Additive4", tables: MORPH_TABLES[26] },
    Wavetable2DPreset { name: "Harmonics1", tables: MORPH_TABLES[27] },
    Wavetable2DPreset { name: "Harmonics2", tables: MORPH_TABLES[28] },
    Wavetable2DPreset { name: "Harmonics3", tables: MORPH_TABLES[29] },
    Wavetable2DPreset { name: "Harmonics4", tables: MORPH_TABLES[30] },
    Wavetable2DPreset { name: "FatSaw1", tables: MORPH_TABLES[31] },
    Wavetable2DPreset { name: "FatSaw2", tables: MORPH_TABLES[32] },
    Wavetable2DPreset { name: "ChipMutate1", tables: MORPH_TABLES[33] },
    Wavetable2DPreset { name: "ChipMutate2", tables: MORPH_TABLES[34] },
];

#[derive(Clone)]
pub struct WavetableOsc2D { runtime: PresetOscillator, params: OscillatorParams }
impl WavetableOsc2D {
    pub fn new(sample_rate: f32) -> Self {
        let params = OscillatorParams { osc_type: OscillatorType::Wavetable, ..Default::default() };
        let mut runtime = PresetOscillator::new(sample_rate); runtime.configure(&params);
        Self { runtime, params }
    }
    pub fn set_preset(&mut self, index: usize) {
        self.params.wavetable = index.min(34) as i32 + 1; self.runtime.configure(&self.params);
    }
    pub fn set_preset_by_name(&mut self, name: &str) -> bool {
        if let Some(index) = WAVETABLE_2D_PRESETS.iter().position(|preset| preset.name == name) {
            self.set_preset(index); true
        } else { false }
    }
    pub fn preset_index(&self) -> usize { (self.params.wavetable - 1).clamp(0, 34) as usize }
    pub fn preset_name(&self) -> &'static str { WAVETABLE_2D_PRESETS[self.preset_index()].name }
    pub fn set_position(&mut self, position: f32) {
        self.params.position = position.clamp(0.0, 1.0); self.runtime.configure(&self.params);
    }
    pub fn position(&self) -> f32 { self.params.position }
    pub fn set_pos_mod_amount(&mut self, amount: f32) {
        self.params.pos_mod = amount; self.runtime.configure(&self.params);
    }
    pub fn set_position_source(&mut self, source: f32) { self.runtime.set_position_source(source); }
    pub fn process_with_mod(&mut self, position: f32) -> f32 {
        self.runtime.set_modulation(OscillatorModulation { position, ..OscillatorModulation::default() });
        self.runtime.process()
    }
}
impl Oscillator for WavetableOsc2D {
    fn process(&mut self) -> f32 { self.process_with_mod(0.0) }
    fn set_frequency(&mut self, frequency: f32) { self.runtime.set_frequency(frequency); }
    fn set_sample_rate(&mut self, rate: f32) { self.runtime.set_sample_rate(rate); }
    fn reset(&mut self) { self.runtime.reset(); }
    fn randomize_phase(&mut self) { self.runtime.randomize_phase(); }
}
impl Default for WavetableOsc2D { fn default() -> Self { Self::new(44100.0) } }
