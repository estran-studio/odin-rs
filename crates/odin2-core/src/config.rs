//! Configuration audio complète, utilisable sans `std`.

use crate::preset::*;
use crate::voice::VoiceConfig;

/// État de patch sans nom, chemins de fichiers ni données JUCE.
/// Toutes les valeurs sont dans les unités physiques d'Odin 2 (dB, Hz, secondes).
#[derive(Clone, Copy, Debug)]
pub struct EngineConfig {
    pub osc1: OscillatorParams,
    pub osc2: OscillatorParams,
    pub osc3: OscillatorParams,
    pub filter1: FilterParams,
    pub filter2: FilterParams,
    pub filter3: FilterParams,
    pub env1: EnvelopeParams,
    pub env2: EnvelopeParams,
    pub env3: EnvelopeParams,
    pub env4: EnvelopeParams,
    pub lfo1: LfoParams,
    pub lfo2: LfoParams,
    pub lfo3: LfoParams,
    pub lfo4: LfoParams,
    pub delay: DelayParams,
    pub reverb: ReverbParams,
    pub chorus: ChorusParams,
    pub phaser: PhaserParams,
    pub flanger: FlangerParams,
    pub distortion: DistortionParams,
    pub mod_matrix: [ModMatrixRow; 9],
    pub arpeggiator: ArpeggiatorParams,
    pub master: f32,
    pub glide: f32,
    pub unison_detune: f32,
    pub unison_width: f32,
    pub unison_voices: i32,
    pub play_mode: PlayMode,
    pub pitchbend_amount: i32,
    pub amp_velocity: f32,
    pub amp_pan: f32,
    pub amp_gain: f32,
    pub xy_x: f32,
    pub xy_y: f32,
    pub modwheel: f32,
    pub pitchbend: f32,
    pub bpm: f32,
    /// Position de delay, phaser, flanger, chorus et reverb respectivement.
    pub fx_positions: [i32; 5],
}

impl Default for EngineConfig {
    fn default() -> Self {
        let mut config = Self {
            osc1: OscillatorParams::default(), osc2: OscillatorParams::default(), osc3: OscillatorParams::default(),
            filter1: FilterParams::default(), filter2: FilterParams::default(), filter3: FilterParams::default(),
            env1: EnvelopeParams::default(), env2: EnvelopeParams::default(), env3: EnvelopeParams::default(), env4: EnvelopeParams::default(),
            lfo1: LfoParams::default(), lfo2: LfoParams::default(), lfo3: LfoParams::default(), lfo4: LfoParams::default(),
            delay: DelayParams::default(), reverb: ReverbParams::default(), chorus: ChorusParams::default(),
            phaser: PhaserParams::default(), flanger: FlangerParams::default(), distortion: DistortionParams::default(),
            mod_matrix: [ModMatrixRow::default(); 9], arpeggiator: ArpeggiatorParams::default(),
            master: -7.0, glide: 0.0, unison_detune: 0.08, unison_width: 0.7, unison_voices: 1,
            play_mode: PlayMode::Poly, pitchbend_amount: 12, amp_velocity: 0.0, amp_pan: 0.0,
            amp_gain: 0.0, xy_x: 0.0, xy_y: 0.0, modwheel: 0.0, pitchbend: 0.0, bpm: 120.0,
            fx_positions: [0, 1, 2, 3, 4],
        };
        config.osc2.osc_type = OscillatorType::None;
        config.osc3.osc_type = OscillatorType::None;
        config.filter2.filter_type = FilterType::None;
        config.filter3.filter_type = FilterType::None;
        crate::parameters::apply_defaults(&mut config);
        config
    }
}

impl EngineConfig {
    pub(crate) fn voice(&self) -> VoiceConfig {
        VoiceConfig {
            oscillators: [self.osc1, self.osc2, self.osc3],
            filters: [self.filter1, self.filter2],
            envelopes: [self.env1, self.env2, self.env3],
            lfos: [self.lfo1, self.lfo2, self.lfo3],
            distortion: self.distortion,
            amp_gain: self.amp_gain,
            pan: self.amp_pan,
            velocity_amount: self.amp_velocity,
            glide: self.glide,
            pitch_bend_range: self.pitchbend_amount as f32,
            play_mode: self.play_mode,
        }
    }

    #[cfg(feature = "std")]
    pub fn from_preset(preset: &OdinPreset) -> Self {
        Self {
            osc1: preset.osc1, osc2: preset.osc2, osc3: preset.osc3,
            filter1: preset.filter1, filter2: preset.filter2, filter3: preset.filter3,
            env1: preset.env1, env2: preset.env2, env3: preset.env3, env4: preset.env4,
            lfo1: preset.lfo1, lfo2: preset.lfo2, lfo3: preset.lfo3, lfo4: preset.lfo4,
            delay: preset.delay, reverb: preset.reverb, chorus: preset.chorus,
            phaser: preset.phaser, flanger: preset.flanger, distortion: preset.distortion,
            mod_matrix: preset.mod_matrix, arpeggiator: preset.arpeggiator,
            master: preset.master, glide: preset.glide, unison_detune: preset.unison_detune,
            unison_width: preset.unison_width, unison_voices: preset.unison_voices,
            play_mode: preset.play_mode, pitchbend_amount: preset.pitchbend_amount,
            amp_velocity: preset.amp_velocity, amp_pan: preset.amp_pan, amp_gain: preset.amp_gain,
            xy_x: preset.xy_x, xy_y: preset.xy_y, modwheel: preset.modwheel, pitchbend: preset.pitchbend,
            bpm: preset.bpm, fx_positions: preset.fx_positions,
        }
    }
}
