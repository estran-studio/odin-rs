//! Chargement fidèle du schéma Odin 2 et audit de consommation.
//! Sources: PluginProcessorReadPatch.cpp, PluginProcessorMigratePatch.cpp,
//! PluginProcessorNonAudioParams.cpp et AudioValueTree.h upstream.
extern crate alloc;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use super::types::*;
use super::value_tree::{ValueTree, ValueTreeError, VariantValue};
use std::fs;
use std::path::Path;

macro_rules! preset_assign {
    ($place:expr, float, $value:expr) => { $place = $value };
    ($place:expr, int, $value:expr) => { $place = $value as i32 };
    ($place:expr, bool, $value:expr) => { $place = $value != 0.0 };
}
macro_rules! audio_assign {
    ($place:expr, float, $value:expr) => { $place = $value };
    ($place:expr, int, $value:expr) => { $place = libm::roundf($value) as i32 };
    ($place:expr, bool, $value:expr) => { $place = $value >= 0.5 };
}
#[derive(Debug, Clone)]
pub struct OdinPreset {
    pub name: String,
    pub version_minor: i32,
    pub version_patch: i32,

    // Oscillators
    pub osc1: OscillatorParams,
    pub osc2: OscillatorParams,
    pub osc3: OscillatorParams,

    // Filters
    pub filter1: FilterParams,
    pub filter2: FilterParams,
    pub filter3: FilterParams,

    // Envelopes
    pub env1: EnvelopeParams, // Amp envelope
    pub env2: EnvelopeParams, // Filter envelope
    pub env3: EnvelopeParams, // Mod envelope
    pub env4: EnvelopeParams, // Global envelope

    // LFOs
    pub lfo1: LfoParams,
    pub lfo2: LfoParams,
    pub lfo3: LfoParams,
    pub lfo4: LfoParams,

    // Effects
    pub delay: DelayParams,
    pub reverb: ReverbParams,
    pub chorus: ChorusParams,
    pub phaser: PhaserParams,
    pub flanger: FlangerParams,
    pub distortion: DistortionParams,

    // Modulation matrix (9 rows)
    pub mod_matrix: [ModMatrixRow; 9],

    // Arpeggiator
    pub arpeggiator: ArpeggiatorParams,

    // Global
    pub master: f32,
    pub glide: f32,
    pub unison_detune: f32,
    pub unison_width: f32,
    pub unison_voices: i32,
    pub play_mode: PlayMode,
    pub pitchbend_amount: i32,

    // Amp section
    pub amp_velocity: f32,
    pub amp_pan: f32,
    pub amp_gain: f32,

    // XY Pad
    pub xy_x: f32,
    pub xy_y: f32,
    /// Version du schéma de patch upstream (actuelle: 5).
    pub patch_migration_version: i32,
    pub bpm: f32,
    /// Positions delay, phaser, flanger, chorus, reverb.
    pub fx_positions: [i32; 5],
    /// Contrôleurs sauvegardés, lus pour audit mais non restaurés par readPatch.
    pub modwheel: f32,
    pub pitchbend: f32,
    pub ui: PresetUiState,
}
/// État d'édition sauvegardé; ce ne sont pas des paramètres audio.
#[derive(Debug, Clone, Default)]
pub struct PresetUiState {
    pub fx_selected: [bool; 5],
    pub lfo_selected: [i32; 2],
    pub env_selected: [i32; 2],
    pub arp_mod_selected: i32,
    pub current_patch_filename: String,
    pub current_patch_directory: String,
    pub preset_soundbank_selected: String,
    pub preset_category_selected: String,
    pub preset_patch_selected: String,
}
/// Les chemins non lus sont produits par les affectations réelles du parseur.
#[derive(Debug, Clone, Default)]
pub struct PresetAudit {
    pub unread: Vec<String>,
    pub migrations: Vec<String>,
    /// Champs explicitement non restaurés par l'amont, jamais cachés.
    pub ignored_by_upstream: Vec<String>,
    pub loaded_parameters: usize,
}
impl Default for OdinPreset {
    fn default() -> Self {
        let mut preset = Self {
            name: "Init Patch".to_string(),
            version_minor: 3,
            version_patch: 4,
            osc1: Default::default(),
            osc2: Default::default(),
            osc3: Default::default(),
            filter1: Default::default(),
            filter2: Default::default(),
            filter3: Default::default(),
            env1: Default::default(),
            env2: Default::default(),
            env3: Default::default(),
            env4: Default::default(),
            lfo1: Default::default(),
            lfo2: Default::default(),
            lfo3: Default::default(),
            lfo4: Default::default(),
            delay: Default::default(),
            reverb: Default::default(),
            chorus: Default::default(),
            phaser: Default::default(),
            flanger: Default::default(),
            distortion: Default::default(),
            mod_matrix: Default::default(),
            arpeggiator: Default::default(),
            master: Default::default(),
            glide: Default::default(),
            unison_detune: Default::default(),
            unison_width: Default::default(),
            unison_voices: 1,
            play_mode: Default::default(),
            pitchbend_amount: 12,
            amp_velocity: Default::default(),
            amp_pan: Default::default(),
            amp_gain: Default::default(),
            xy_x: Default::default(),
            xy_y: Default::default(),
            patch_migration_version: 5,
            bpm: 120.0,
            fx_positions: [0, 1, 3, 2, 4],
            modwheel: Default::default(),
            pitchbend: Default::default(),
            ui: Default::default(),
        };
        preset.ui.fx_selected[0] = true;
        preset.ui.lfo_selected = [1, 1];
        preset.ui.env_selected = [1, 1];
        preset.ui.current_patch_filename = "Init Patch".to_string();
        preset.osc2.osc_type = OscillatorType::None;
        preset.osc3.osc_type = OscillatorType::None;
        preset.filter2.filter_type = FilterType::None;
        preset.filter3.filter_type = FilterType::None;
        macro_rules! defaults {
            ($p:ident; $(($id:literal, $($field:ident $( [ $index:literal ] )?).+, $kind:ident, $min:literal, $max:literal, $default:literal),)*) => {
                $(preset_assign!($p.$($field $( [ $index ] )?).+, $kind, $default as f32);)*
            };
        }
        crate::odin_audio_parameters!(defaults, preset);
        preset
    }
}
impl OdinPreset {
    /// Load preset from file path
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ValueTreeError> {
        let data = fs::read(path).map_err(ValueTreeError::Io)?;
        Self::from_bytes(&data)
    }

    /// Perform a 2D scatter morph between multiple presets
    ///
    /// # Arguments
    /// * `sources` - Valid presets with their (x, y) coordinates in range [-1.0, 1.0]
    /// * `x` - Target x coordinate [-1.0, 1.0]
    /// * `y` - Target y coordinate [-1.0, 1.0]
    ///
    /// Uses inverse distance weighting to interpolate between an arbitrary number of presets.
    pub fn morph_2d(sources: &[(OdinPreset, f32, f32)], x: f32, y: f32) -> Self {
        if sources.is_empty() {
            return Self::default();
        }

        // Just one preset? Return it directly
        if sources.len() == 1 {
            return sources[0].0.clone();
        }

        // Check for exact matches (avoid division by zero)
        for (preset, px, py) in sources {
            let dx = x - px;
            let dy = y - py;
            if dx * dx + dy * dy < 0.00001 {
                return preset.clone();
            }
        }

        // Calculate weights using Inverse Distance Weighting (IDW)
        // weight = 1 / distance^p (using p=2 for smooth transitions)
        let mut weights = Vec::with_capacity(sources.len());
        let mut total_weight = 0.0;

        for (_, px, py) in sources {
            let dx = x - px;
            let dy = y - py;
            let dist_sq = dx * dx + dy * dy;
            let weight = 1.0 / dist_sq.max(0.00001); // Avoid zero division just in case
            weights.push(weight);
            total_weight += weight;
        }

        // Normalize weights
        for w in &mut weights {
            *w /= total_weight;
        }

        // Mix presets
        let preset_refs: Vec<&OdinPreset> = sources.iter().map(|(p, _, _)| p).collect();
        Self::mix_presets(&preset_refs, &weights)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, ValueTreeError> {
        Self::from_value_tree(&ValueTree::from_bytes(data)?)
    }
    pub fn from_value_tree(tree: &ValueTree) -> Result<Self, ValueTreeError> {
        Self::from_value_tree_audited(tree).map(|(preset, _)| preset)
    }
    /// Charge le patch et rend les chemins réellement non consommés.
    /// Les champs inconnus ne sont ni whitelistés ni traités comme consommés
    /// simplement parce que le ValueTree les a conservés.
    pub fn from_value_tree_audited(tree: &ValueTree) -> Result<(Self, PresetAudit), ValueTreeError> {
        let mut preset = Self::default();
        let mut reader = PatchReader::new(tree);
        let mut migrations = Vec::new();
        let migration_version = reader.number("misc", "patch_migration_version").unwrap_or(0.0) as i32;
        preset.patch_migration_version = migration_version;
        if migration_version < 5 {
            migrations.push("readPatch: init baseline for migration version < 5".to_string());
            preset.set_audio_param("fil2_freq", 2000.0003662109375);
            preset.set_audio_param("fil3_freq", 2000.0003662109375);
            preset.set_audio_param("env1_decay", 0.9999998807907104);
            preset.set_audio_param("env4_decay", 0.9999998807907104);
            preset.set_audio_param("lfo1_freq", 1.0000003576278687);
            preset.set_audio_param("lfo2_freq", 1.0000003576278687);
            preset.set_audio_param("lfo3_freq", 1.0000003576278687);
            preset.set_audio_param("lfo4_freq", 1.0000003576278687);
            preset.set_audio_param("delay_wet", 0.7000000476837158);
            preset.set_audio_param("rev_eqfreq", 1000.0001831054688);
            preset.set_audio_param("rev_hf_damp", 6000.0009765625);
            preset.set_audio_param("phaser_freq", 3999.999755859375);
            preset.set_audio_param("phaser_rate", 0.2500000596046448);
            preset.set_audio_param("flanger_rate", 0.15000002086162567);
            preset.set_audio_param("flanger_feedback", 0.5999999046325684);
            preset.set_audio_param("chorus_rate", 0.15000002086162567);
        }
        macro_rules! number {
            ($section:literal, $key:expr, $place:expr, $kind:ident) => {
                if let Some(value) = reader.number($section, $key) { preset_assign!($place, $kind, value); }
            };
        }
        macro_rules! enumeration {
            ($section:literal, $key:expr, $place:expr, $ty:ty) => {
                if let Some(value) = reader.number($section, $key) { $place = <$ty>::from(value as i32); }
            };
        }
        number!("misc", "version_minor", preset.version_minor, int);
        number!("misc", "version_patch", preset.version_patch, int);
        number!("misc", "BPM", preset.bpm, float);
        number!("misc", "pitchbend_amount", preset.pitchbend_amount, int);
        number!("misc", "unison_voices", preset.unison_voices, int);
        number!("misc", "dist_on", preset.distortion.on, bool);
        number!("misc", "arp_octaves", preset.arpeggiator.octaves, int);
        number!("misc", "arp_steps", preset.arpeggiator.steps, int);
        number!("misc", "arp_gate", preset.arpeggiator.gate, int);
        number!("misc", "arp_synctime_numerator", preset.arpeggiator.sync_numerator, int);
        number!("misc", "arp_synctime_denominator", preset.arpeggiator.sync_denominator, int);
        number!("misc", "env_left_selected", preset.ui.env_selected[0], int);
        number!("misc", "env_right_selected", preset.ui.env_selected[1], int);
        number!("misc", "arp_mod_selected", preset.ui.arp_mod_selected, int);
        number!("lfo", "lfo_left_selected", preset.ui.lfo_selected[0], int);
        number!("lfo", "lfo_right_selected", preset.ui.lfo_selected[1], int);
        number!("misc", "fil1_comb_polarity", preset.filter1.comb_polarity, bool);
        number!("misc", "fil1_vowel_left", preset.filter1.vowel_left, int);
        number!("misc", "fil1_vowel_right", preset.filter1.vowel_right, int);
        number!("osc", "osc1_wavetable", preset.osc1.wavetable, int);
        number!("osc", "osc1_vec_a", preset.osc1.vec_a, int);
        number!("osc", "osc1_vec_b", preset.osc1.vec_b, int);
        number!("osc", "osc1_vec_c", preset.osc1.vec_c, int);
        number!("osc", "osc1_vec_d", preset.osc1.vec_d, int);
        number!("osc", "osc1_chipwave", preset.osc1.chipwave, int);
        number!("osc", "osc1_modulator_wave", preset.osc1.modulator_wave, int);
        number!("osc", "osc1_carrier_wave", preset.osc1.carrier_wave, int);
        number!("osc", "osc1_mod_source", preset.osc1.mod_source, int);
        number!("osc", "osc1_carrier_ratio", preset.osc1.carrier_ratio, int);
        number!("osc", "osc1_modulator_ratio", preset.osc1.modulator_ratio, int);
        number!("misc", "fil2_comb_polarity", preset.filter2.comb_polarity, bool);
        number!("misc", "fil2_vowel_left", preset.filter2.vowel_left, int);
        number!("misc", "fil2_vowel_right", preset.filter2.vowel_right, int);
        number!("osc", "osc2_wavetable", preset.osc2.wavetable, int);
        number!("osc", "osc2_vec_a", preset.osc2.vec_a, int);
        number!("osc", "osc2_vec_b", preset.osc2.vec_b, int);
        number!("osc", "osc2_vec_c", preset.osc2.vec_c, int);
        number!("osc", "osc2_vec_d", preset.osc2.vec_d, int);
        number!("osc", "osc2_chipwave", preset.osc2.chipwave, int);
        number!("osc", "osc2_modulator_wave", preset.osc2.modulator_wave, int);
        number!("osc", "osc2_carrier_wave", preset.osc2.carrier_wave, int);
        number!("osc", "osc2_mod_source", preset.osc2.mod_source, int);
        number!("osc", "osc2_carrier_ratio", preset.osc2.carrier_ratio, int);
        number!("osc", "osc2_modulator_ratio", preset.osc2.modulator_ratio, int);
        number!("misc", "fil3_comb_polarity", preset.filter3.comb_polarity, bool);
        number!("misc", "fil3_vowel_left", preset.filter3.vowel_left, int);
        number!("misc", "fil3_vowel_right", preset.filter3.vowel_right, int);
        number!("osc", "osc3_wavetable", preset.osc3.wavetable, int);
        number!("osc", "osc3_vec_a", preset.osc3.vec_a, int);
        number!("osc", "osc3_vec_b", preset.osc3.vec_b, int);
        number!("osc", "osc3_vec_c", preset.osc3.vec_c, int);
        number!("osc", "osc3_vec_d", preset.osc3.vec_d, int);
        number!("osc", "osc3_chipwave", preset.osc3.chipwave, int);
        number!("osc", "osc3_modulator_wave", preset.osc3.modulator_wave, int);
        number!("osc", "osc3_carrier_wave", preset.osc3.carrier_wave, int);
        number!("osc", "osc3_mod_source", preset.osc3.mod_source, int);
        number!("osc", "osc3_carrier_ratio", preset.osc3.carrier_ratio, int);
        number!("osc", "osc3_modulator_ratio", preset.osc3.modulator_ratio, int);
        number!("lfo", "lfo1_sync", preset.lfo1.sync, bool);
        number!("lfo", "lfo1_synctime_numerator", preset.lfo1.sync_numerator, int);
        number!("lfo", "lfo1_synctime_denominator", preset.lfo1.sync_denominator, int);
        number!("lfo", "lfo2_sync", preset.lfo2.sync, bool);
        number!("lfo", "lfo2_synctime_numerator", preset.lfo2.sync_numerator, int);
        number!("lfo", "lfo2_synctime_denominator", preset.lfo2.sync_denominator, int);
        number!("lfo", "lfo3_sync", preset.lfo3.sync, bool);
        number!("lfo", "lfo3_synctime_numerator", preset.lfo3.sync_numerator, int);
        number!("lfo", "lfo3_synctime_denominator", preset.lfo3.sync_denominator, int);
        number!("lfo", "lfo4_sync", preset.lfo4.sync, bool);
        number!("lfo", "lfo4_synctime_numerator", preset.lfo4.sync_numerator, int);
        number!("lfo", "lfo4_synctime_denominator", preset.lfo4.sync_denominator, int);
        number!("fx", "delay_position", preset.fx_positions[0], int);
        number!("fx", "delay_selected", preset.ui.fx_selected[0], bool);
        number!("fx", "delay_sync", preset.delay.sync, bool);
        number!("fx", "delay_synctime_numerator", preset.delay.sync_numerator, int);
        number!("fx", "delay_synctime_denominator", preset.delay.sync_denominator, int);
        number!("fx", "phaser_position", preset.fx_positions[1], int);
        number!("fx", "phaser_selected", preset.ui.fx_selected[1], bool);
        number!("fx", "phaser_sync", preset.phaser.sync, bool);
        number!("fx", "phaser_synctime_numerator", preset.phaser.sync_numerator, int);
        number!("fx", "phaser_synctime_denominator", preset.phaser.sync_denominator, int);
        number!("fx", "flanger_position", preset.fx_positions[2], int);
        number!("fx", "flanger_selected", preset.ui.fx_selected[2], bool);
        number!("fx", "flanger_sync", preset.flanger.sync, bool);
        number!("fx", "flanger_synctime_numerator", preset.flanger.sync_numerator, int);
        number!("fx", "flanger_synctime_denominator", preset.flanger.sync_denominator, int);
        number!("fx", "chorus_position", preset.fx_positions[3], int);
        number!("fx", "chorus_selected", preset.ui.fx_selected[3], bool);
        number!("fx", "chorus_sync", preset.chorus.sync, bool);
        number!("fx", "chorus_synctime_numerator", preset.chorus.sync_numerator, int);
        number!("fx", "chorus_synctime_denominator", preset.chorus.sync_denominator, int);
        number!("fx", "reverb_position", preset.fx_positions[4], int);
        number!("fx", "reverb_selected", preset.ui.fx_selected[4], bool);
        enumeration!("misc", "legato", preset.play_mode, PlayMode);
        enumeration!("misc", "dist_algo", preset.distortion.algorithm, DistortionAlgo);
        enumeration!("misc", "arp_direction", preset.arpeggiator.direction, ArpDirection);
        enumeration!("misc", "fil1_type", preset.filter1.filter_type, FilterType);
        enumeration!("osc", "osc1_type", preset.osc1.osc_type, OscillatorType);
        enumeration!("osc", "osc1_analog_wave", preset.osc1.analog_wave, AnalogWaveform);
        enumeration!("misc", "fil2_type", preset.filter2.filter_type, FilterType);
        enumeration!("osc", "osc2_type", preset.osc2.osc_type, OscillatorType);
        enumeration!("osc", "osc2_analog_wave", preset.osc2.analog_wave, AnalogWaveform);
        enumeration!("misc", "fil3_type", preset.filter3.filter_type, FilterType);
        enumeration!("osc", "osc3_type", preset.osc3.osc_type, OscillatorType);
        enumeration!("osc", "osc3_analog_wave", preset.osc3.analog_wave, AnalogWaveform);
        enumeration!("lfo", "lfo1_wave", preset.lfo1.waveform, LfoWaveform);
        enumeration!("lfo", "lfo2_wave", preset.lfo2.waveform, LfoWaveform);
        enumeration!("lfo", "lfo3_wave", preset.lfo3.waveform, LfoWaveform);
        enumeration!("lfo", "lfo4_wave", preset.lfo4.waveform, LfoWaveform);
        if let Some(value) = reader.text("misc", "patch_name") { preset.name = value; }
        if let Some(value) = reader.text("misc", "current_patch_filename") { preset.ui.current_patch_filename = value; }
        if let Some(value) = reader.text("misc", "current_patch_directory") { preset.ui.current_patch_directory = value; }
        if let Some(value) = reader.text("misc", "preset_soundbank_selected") { preset.ui.preset_soundbank_selected = value; }
        if let Some(value) = reader.text("misc", "preset_category_selected") { preset.ui.preset_category_selected = value; }
        if let Some(value) = reader.text("misc", "preset_patch_selected") { preset.ui.preset_patch_selected = value; }
        for (index, oscillator) in [&mut preset.osc1, &mut preset.osc2, &mut preset.osc3].into_iter().enumerate() {
            let n = index + 1;
            for (kind, values) in [("wavedraw", oscillator.wavedraw.as_mut_slice()),
                ("chipdraw", oscillator.chipdraw.as_mut_slice()), ("specdraw", oscillator.specdraw.as_mut_slice())] {
                // readPatch regenerates an absent table; defaults above are the
                // exact sine / bipolar 0.875 pulse / first partial generators.
                for (i, sample) in values.iter_mut().enumerate() {
                    if let Some(value) = reader.number("draw", &format!("osc{n}_{kind}_values_{i}")) { *sample = value; }
                }
            }
        }
        for (row, params) in preset.mod_matrix.iter_mut().enumerate() {
            number!("mod", &format!("source_row_{row}"), params.source, int);
            number!("mod", &format!("dest_1_row_{row}"), params.dest1, int);
            number!("mod", &format!("dest_2_row_{row}"), params.dest2, int);
            number!("mod", &format!("scale_row_{row}"), params.scale, int);
            number!("mod", &format!("amount_0_row_{row}"), params.dest1_amount, float);
            number!("mod", &format!("amount_1_row_{row}"), params.dest2_amount, float);
            number!("mod", &format!("amount_2_row_{row}"), params.scale_amount, float);
        }
        // MigratePatch HACK #1 always runs, even on modern patches: old audio
        // ratios and pitchbend range override the non-audio properties.
        for (i, oscillator) in [&mut preset.osc1, &mut preset.osc2, &mut preset.osc3].into_iter().enumerate() {
            for (suffix, target) in [("carrier_ratio", &mut oscillator.carrier_ratio), ("modulator_ratio", &mut oscillator.modulator_ratio)] {
                let key = format!("osc{}_{suffix}", i + 1);
                if let Some(value) = reader.param(&key) {
                    *target = value as i32; migrations.push(format!("PARAM/{key} -> osc/{key}"));
                }
            }
        }
        if let Some(value) = reader.param("pitchbend_amount") {
            preset.pitchbend_amount = value as i32;
            migrations.push("PARAM/pitchbend_amount -> misc/pitchbend_amount".to_string());
        }
        reader.ignore_upstream("amp_width", "obsolete parameter removed in patch schema 2");
        reader.ignore_upstream("modwheel", "readPatch::checkLoadParameter returns false");
        reader.ignore_upstream("pitchbend", "readPatch::checkLoadParameter returns false");
        for (name, value) in &tree.params {
            // readPatch::checkLoadParameter rejette ces trois noms : le patch ne modifie jamais
            // la molette ni le pitch bend, qui restent des contrôleurs de l'hôte.
            if matches!(name.as_str(), "modwheel" | "pitchbend" | "amp_width") { continue; }
            if preset.set_audio_param(name, *value as f32) { reader.param(name); }
        }
        Ok((preset, reader.finish(migrations)))
    }
    /// Paramètre audio connu, en unités physiques. Pas d'allocation.
    pub fn set_audio_param(&mut self, name: &str, value: f32) -> bool {
        macro_rules! set {
            ($p:ident $name:ident $value:ident; $(($id:literal, $($field:ident $( [ $index:literal ] )?).+, $kind:ident, $min:literal, $max:literal, $default:literal),)*) => {
                match $name { $($id => {
                    let bounded = $value.clamp($min, $max);
                    audio_assign!($p.$($field $( [ $index ] )?).+, $kind, bounded);
                    true
                },)* _ => false }
            };
        }
        crate::odin_audio_parameters!(set, self, name, value)
    }
    pub fn audio_param(&self, name: &str) -> Option<f32> {
        // Bool conversion is separate; numeric values must not be truncated.
        macro_rules! scalar { ($v:expr, bool) => { if $v { 1.0 } else { 0.0 } }; ($v:expr, $kind:ident) => { $v as f32 }; }
        macro_rules! get_scalar {
            ($p:ident $name:ident; $(($id:literal, $($field:ident $( [ $index:literal ] )?).+, $kind:ident, $min:literal, $max:literal, $default:literal),)*) => {
                match $name { $($id => Some(scalar!($p.$($field $( [ $index ] )?).+, $kind)),)* _ => None }
            };
        }
        crate::odin_audio_parameters!(get_scalar, self, name)
    }
}
struct PatchReader<'a> {
    tree: &'a ValueTree,
    consumed: BTreeSet<String>,
    ignored_by_upstream: Vec<String>,
}
impl<'a> PatchReader<'a> {
    fn new(tree: &'a ValueTree) -> Self {
        Self { tree, consumed: BTreeSet::new(), ignored_by_upstream: Vec::new() }
    }
    fn section(&self, section: &str) -> &BTreeMap<String, VariantValue> {
        match section { "misc" => &self.tree.misc, "osc" => &self.tree.osc,
            "fx" => &self.tree.fx, "lfo" => &self.tree.lfo, "mod" => &self.tree.mod_matrix,
            "draw" => &self.tree.draw, _ => unreachable!() }
    }
    fn number(&mut self, section: &str, name: &str) -> Option<f32> {
        let value = self.section(section).get(name)?.as_f32()?;
        self.consumed.insert(format!("{section}/{name}")); Some(value)
    }
    fn text(&mut self, section: &str, name: &str) -> Option<String> {
        let value = self.section(section).get(name)?.as_string()?.to_string();
        self.consumed.insert(format!("{section}/{name}")); Some(value)
    }
    fn param(&mut self, name: &str) -> Option<f32> {
        let value = *self.tree.params.get(name)? as f32;
        self.consumed.insert(format!("PARAM/{name}")); Some(value)
    }
    fn ignore_upstream(&mut self, name: &str, reason: &str) {
        if self.param(name).is_some() {
            self.ignored_by_upstream.push(format!("PARAM/{name}: {reason}"));
        }
    }
    fn finish(self, migrations: Vec<String>) -> PresetAudit {
        let mut unread = self.tree.unmapped.clone();
        for (section, values) in [("misc", &self.tree.misc), ("osc", &self.tree.osc),
            ("fx", &self.tree.fx), ("lfo", &self.tree.lfo), ("mod", &self.tree.mod_matrix), ("draw", &self.tree.draw)] {
            for key in values.keys() {
                let path = format!("{section}/{key}");
                if !self.consumed.contains(&path) { unread.push(path); }
            }
        }
        for key in self.tree.params.keys() {
            let path = format!("PARAM/{key}");
            if !self.consumed.contains(&path) { unread.push(path); }
        }
        unread.sort(); unread.dedup();
        let loaded_parameters = self.consumed.iter()
            .filter(|p| p.starts_with("PARAM/") && !matches!(p.as_str(), "PARAM/amp_width" | "PARAM/modwheel" | "PARAM/pitchbend"))
            .count();
        PresetAudit { unread, migrations, loaded_parameters, ignored_by_upstream: self.ignored_by_upstream }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_serialized_ids_are_not_dropdown_indices() {
        assert_eq!(OscillatorType::from(1), OscillatorType::None);
        assert_eq!(OscillatorType::from(2), OscillatorType::Analog);
        assert_eq!(OscillatorType::from(20), OscillatorType::Wavedraw);
        assert_eq!(FilterType::from(1), FilterType::None);
        assert_eq!(FilterType::from(25), FilterType::KorgHp);
        assert_eq!(LfoWaveform::from(2), LfoWaveform::Triangle);
        assert_eq!(LfoWaveform::from(7), LfoWaveform::SampleAndHold);
        assert_eq!(DistortionAlgo::from(2), DistortionAlgo::Fold);
        assert_eq!(ArpDirection::from(60), ArpDirection::CrawlUp);
    }

    #[test]
    fn omitted_values_use_upstream_defaults_and_tables() {
        let (preset, audit) = OdinPreset::from_value_tree_audited(&ValueTree::default()).unwrap();
        assert_eq!(preset.master, -7.0);
        assert_eq!(preset.filter1.frequency, 20000.0);
        assert!(preset.filter1.osc1_input);
        assert!(!preset.filter2.osc1_input);
        assert!(preset.filter2.fil1_input);
        assert_eq!(preset.filter2.filter_type, FilterType::None);
        assert_eq!(preset.env2.decay, 0.8);
        assert_eq!(preset.env2.sustain, 0.0);
        assert_eq!(preset.osc1.pulsewidth, 0.5);
        assert_eq!(preset.osc1.chipdraw[15], 0.875);
        assert_eq!(preset.osc1.chipdraw[16], -0.875);
        assert_eq!(preset.osc1.specdraw[0], 1.0);
        assert!(audit.unread.is_empty());
        assert!(!audit.migrations.is_empty());
    }

    #[test]
    fn legacy_audio_ratios_override_nonaudio_even_in_modern_patch() {
        let mut tree = ValueTree::default();
        tree.misc.insert("patch_migration_version".to_string(), VariantValue::Int(5));
        tree.osc.insert("osc1_carrier_ratio".to_string(), VariantValue::Int(2));
        tree.params.insert("osc1_carrier_ratio".to_string(), 7.0);
        tree.params.insert("osc3_modulator_ratio".to_string(), 9.0);
        tree.params.insert("pitchbend_amount".to_string(), 24.0);
        let (preset, audit) = OdinPreset::from_value_tree_audited(&tree).unwrap();
        assert_eq!(preset.osc1.carrier_ratio, 7);
        assert_eq!(preset.osc3.modulator_ratio, 9);
        assert_eq!(preset.pitchbend_amount, 24);
        assert_eq!(audit.migrations.len(), 3);
        assert!(audit.unread.is_empty());
    }

    #[test]
    fn audit_distinguishes_consumed_ignored_and_unknown_parameters() {
        let mut tree = ValueTree::default();
        tree.params.insert("osc1_vol".to_string(), -13.0);
        tree.params.insert("amp_width".to_string(), 0.5);
        tree.params.insert("invented_future_parameter".to_string(), 0.5);
        tree.draw.insert("osc1_wavedraw_values_192".to_string(), VariantValue::Double(0.25));
        tree.draw.insert("osc1_wavedraw_values_193".to_string(), VariantValue::Double(0.5));
        let (preset, audit) = OdinPreset::from_value_tree_audited(&tree).unwrap();
        assert_eq!(preset.osc1.volume, -13.0);
        assert_eq!(preset.osc1.wavedraw[192], 0.25);
        assert_eq!(audit.unread, ["PARAM/invented_future_parameter", "draw/osc1_wavedraw_values_193"]);
        assert_eq!(audit.ignored_by_upstream.len(), 1);
        assert_eq!(audit.loaded_parameters, 1);
    }
}
