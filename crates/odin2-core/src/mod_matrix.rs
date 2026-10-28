//! Matrice Odin 2, sans allocation et avec les identifiants sérialisés amont.
//!
//! Références: `Source/gui/ModMatrix.cpp:23-110` (équations), `120-1029`
//! (identifiants), `PluginProcessorSetModPointers.cpp:25-55,73-357` (consommateurs),
//! `PluginProcessorProcess.cpp:82-161` (ordre de calcul).
//!
//! Pour chaque échantillon, calculer les destinations **avant** de remplacer les
//! sources DSP: enveloppes, LFO, sorties oscillateurs et filtres sont celles de
//! l'échantillon précédent. Les contrôleurs MIDI peuvent déjà avoir changé.
//! Une destination globale lit la dernière voix déclenchée, jamais la somme des
//! voix. Le pad XY est calculé avant la matrice avec sa destination précédente.
//! Les sorties oscillateur/filtre sources comprennent leur gain modulé.
//!
//! Pour une route: `source * amount * abs(amount) * scale_factor`. Sans source
//! scale, le facteur vaut 1; sinon il vaut `1 + (scale - 1) * scale_amount` pour
//! `scale_amount >= 0`, ou `1 + abs(scale) * scale_amount` pour `scale_amount < 0`. Les deux
//! destinations et les neuf lignes s'additionnent, sans écrêtage dans la matrice.

use crate::constants::MOD_MATRIX_ROWS;
pub use crate::preset::ModMatrixRow;

macro_rules! sources {
    ($($name:ident = $id:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        #[repr(u16)]
        pub enum ModSource { #[default] None = 0, $($name = $id),+ }
        impl ModSource {
            pub const fn from_id(id: i32) -> Option<Self> {
                match id { 0 => Some(Self::None), $($id => Some(Self::$name),)+ _ => None }
            }
        }
    };
}
sources! {
    Osc1 = 100, Osc2 = 101, Osc3 = 102, Filter1 = 110, Filter2 = 111,
    Env1 = 200, Env2 = 201, Env3 = 202, Env4 = 203,
    Lfo1 = 300, Lfo2 = 301, Lfo3 = 302, Lfo4 = 303,
    X = 400, Y = 401, ModWheel = 402, PitchWheel = 403,
    Note = 404, Velocity = 405, Aftertouch = 406, Breath = 407,
    ChannelPressure = 409, UnisonPosition = 420, ArpMod1 = 430, ArpMod2 = 435,
    SustainPedal = 440, SoftPedal = 450, Random = 999, Constant = 1000,
}

macro_rules! destinations {
    ($($name:ident = $id:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u16)]
        pub enum ModDest { None = 0, $($name = $id),+ }
        #[derive(Clone, Copy)]
        enum DestinationSlot { $($name),+ }
        const DESTINATION_IDS: &[i32] = &[$($id),+];
        const DESTINATION_COUNT: usize = DESTINATION_IDS.len();
        fn destination_slot(id: i32) -> Option<usize> {
            match id { $($id => Some(DestinationSlot::$name as usize),)+ _ => None }
        }
        impl ModDest {
            pub const fn from_id(id: i32) -> Option<Self> {
                match id { 0 => Some(Self::None), $($id => Some(Self::$name),)+ _ => None }
            }
        }
    };
}
destinations! {
    Osc1Pitch = 2, Osc1PitchLinear = 3, Osc1Amp = 4, Osc1Pwm = 10,
    Osc1Position = 20, Osc1Detune = 21, Osc1Spread = 22,
    Osc1X = 30, Osc1Y = 31, Osc1ArpSpeed = 40,
    Osc1FmAmount = 50, Osc1CarrierRatio = 51, Osc1ModulatorRatio = 52,
    Osc1Lp = 60, Osc1Hp = 61,
    Osc2Pitch = 102, Osc2PitchLinear = 103, Osc2Amp = 104, Osc2Pwm = 110,
    Osc2Position = 120, Osc2Detune = 121, Osc2Spread = 122,
    Osc2X = 130, Osc2Y = 131, Osc2ArpSpeed = 140,
    Osc2FmAmount = 150, Osc2CarrierRatio = 151, Osc2ModulatorRatio = 152,
    Osc2Lp = 160, Osc2Hp = 161,
    Osc3Pitch = 202, Osc3PitchLinear = 203, Osc3Amp = 204, Osc3Pwm = 210,
    Osc3Position = 220, Osc3Detune = 221, Osc3Spread = 222,
    Osc3X = 230, Osc3Y = 231, Osc3ArpSpeed = 240,
    Osc3FmAmount = 250, Osc3CarrierRatio = 251, Osc3ModulatorRatio = 252,
    Osc3Lp = 260, Osc3Hp = 261,
    Filter1Cutoff = 301, Filter1Resonance = 302, Filter1Gain = 303,
    Filter1Envelope = 304, Filter1Velocity = 305, Filter1Keyboard = 306,
    Filter1Saturation = 307, Filter1Sem = 310, Filter1Formant = 320, Filter1Ring = 330,
    Filter2Cutoff = 401, Filter2Resonance = 402, Filter2Gain = 403,
    Filter2Envelope = 404, Filter2Velocity = 405, Filter2Keyboard = 406,
    Filter2Saturation = 407, Filter2Sem = 410, Filter2Formant = 420, Filter2Ring = 430,
    Filter3Cutoff = 501, Filter3Resonance = 502, Filter3Gain = 503,
    Filter3Envelope = 504, Filter3Velocity = 505, Filter3Keyboard = 506,
    Filter3Saturation = 507, Filter3Sem = 510, Filter3Formant = 520, Filter3Ring = 530,
    Env1Attack = 601, Env1Decay = 602, Env1Sustain = 603, Env1Release = 604,
    Env2Attack = 611, Env2Decay = 612, Env2Sustain = 613, Env2Release = 614,
    Env3Attack = 621, Env3Decay = 622, Env3Sustain = 623, Env3Release = 624,
    Env4Attack = 631, Env4Decay = 632, Env4Sustain = 633, Env4Release = 634,
    Lfo1Rate = 651, Lfo2Rate = 661, Lfo3Rate = 671, Lfo4Rate = 681,
    DelayTime = 701, DelayFeedback = 702, DelayHp = 703, DelayDry = 704, DelayWet = 705,
    PhaserRate = 751, PhaserAmount = 752, PhaserFrequency = 753,
    PhaserFeedback = 754, PhaserMix = 755,
    FlangerRate = 801, FlangerAmount = 802, FlangerFeedback = 803, FlangerMix = 804,
    ChorusRate = 851, ChorusAmount = 852, ChorusFeedback = 853, ChorusMix = 854,
    Volume = 900, Pan = 901, AmpVelocity = 902, DistortionBoost = 950,
    DistortionMix = 951, ArpSpeed = 970, ArpGate = 973, X = 980, Y = 982,
    Glide = 999, Master = 1000,
}

impl ModDest {
    pub const fn is_global(self) -> bool {
        matches!(self as u16, 501..=530 | 631..=634 | 681 | 701..=854 | 970 | 973 | 980 | 982 | 999 | 1000)
    }
}

/// Sources de voix conservées entre deux échantillons (MIDI key normalisé 0..1).
#[derive(Debug, Clone, Copy, Default)]
pub struct ModSources {
    pub osc: [f32; 3],
    pub filter: [f32; 2],
    pub env: [f32; 3],
    pub lfo: [f32; 3],
    pub note: f32,
    pub velocity: f32,
    pub aftertouch: f32,
    pub unison_position: f32,
    pub random: f32,
    pub arp_mod: [f32; 2],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalSources {
    pub env: f32,
    pub lfo: f32,
    pub x: f32,
    pub y: f32,
    pub modwheel: f32,
    pub pitchwheel: f32,
    pub breath: f32,
    pub channel_pressure: f32,
    pub sustain_pedal: f32,
    pub soft_pedal: f32,
}

impl ModSources {
    pub fn get(&self, source: ModSource, global: &GlobalSources) -> f32 {
        match source {
            ModSource::None => 0.0,
            ModSource::Osc1 => self.osc[0], ModSource::Osc2 => self.osc[1], ModSource::Osc3 => self.osc[2],
            ModSource::Filter1 => self.filter[0], ModSource::Filter2 => self.filter[1],
            ModSource::Env1 => self.env[0], ModSource::Env2 => self.env[1], ModSource::Env3 => self.env[2],
            ModSource::Env4 => global.env,
            ModSource::Lfo1 => self.lfo[0], ModSource::Lfo2 => self.lfo[1], ModSource::Lfo3 => self.lfo[2],
            ModSource::Lfo4 => global.lfo,
            ModSource::X => global.x, ModSource::Y => global.y,
            ModSource::ModWheel => global.modwheel, ModSource::PitchWheel => global.pitchwheel,
            ModSource::Note => self.note, ModSource::Velocity => self.velocity,
            ModSource::Aftertouch => self.aftertouch, ModSource::Breath => global.breath,
            ModSource::ChannelPressure => global.channel_pressure,
            ModSource::UnisonPosition => self.unison_position,
            ModSource::ArpMod1 => self.arp_mod[0], ModSource::ArpMod2 => self.arp_mod[1],
            ModSource::SustainPedal => global.sustain_pedal, ModSource::SoftPedal => global.soft_pedal,
            ModSource::Random => self.random, ModSource::Constant => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OscModulation {
    pub pitch_exponential: f32, pub pitch_linear: f32, pub vol: f32,
    pub pulse_width: f32, pub position: f32, pub detune: f32, pub spread: f32,
    pub x: f32, pub y: f32, pub arp_speed: f32, pub fm_amount: f32,
    pub carrier_ratio: f32, pub modulator_ratio: f32, pub lp_freq: f32, pub hp_freq: f32,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct FilterModulation {
    pub freq: f32, pub res: f32, pub gain: f32, pub env_amount: f32,
    pub vel_amount: f32, pub kbd_amount: f32, pub saturation: f32,
    pub sem_transition: f32, pub formant_transition: f32, pub ringmod_amount: f32,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct EnvelopeModulation {
    pub attack: f32, pub decay: f32, pub sustain: f32, pub release: f32,
}

impl From<FilterModulation> for crate::dsp::filters::FilterModulation {
    fn from(value: FilterModulation) -> Self {
        Self { frequency: value.freq, resonance: value.res, gain: value.gain,
            saturation: value.saturation, envelope: value.env_amount,
            velocity: value.vel_amount, keyboard: value.kbd_amount,
            sem_transition: value.sem_transition, formant_transition: value.formant_transition,
            ring_amount: value.ringmod_amount }
    }
}

/// Destinations compactes: aucun tableau indexé jusqu'à l'ID 1000.
#[derive(Debug, Clone, Copy)]
pub struct ModulationValues { values: [f32; DESTINATION_COUNT] }
impl Default for ModulationValues {
    fn default() -> Self { Self { values: [0.0; DESTINATION_COUNT] } }
}
impl ModulationValues {
    /// Les IDs sont vérifiés lors de `configure`; l'ID 0 désigne « aucune destination ».
    pub fn get(&self, id: i32) -> f32 {
        destination_slot(id).map_or(0.0, |slot| self.values[slot])
    }
    pub fn osc(&self, index: usize) -> OscModulation {
        assert!(index < 3);
        let base = index as i32 * 100;
        OscModulation { pitch_exponential: self.get(base + 2), pitch_linear: self.get(base + 3),
            vol: self.get(base + 4), pulse_width: self.get(base + 10), position: self.get(base + 20),
            detune: self.get(base + 21), spread: self.get(base + 22), x: self.get(base + 30),
            y: self.get(base + 31), arp_speed: self.get(base + 40), fm_amount: self.get(base + 50),
            carrier_ratio: self.get(base + 51), modulator_ratio: self.get(base + 52),
            lp_freq: self.get(base + 60), hp_freq: self.get(base + 61) }
    }
    pub fn filter(&self, index: usize) -> FilterModulation {
        assert!(index < 3);
        let base = 300 + index as i32 * 100;
        FilterModulation { freq: self.get(base + 1), res: self.get(base + 2), gain: self.get(base + 3),
            env_amount: self.get(base + 4), vel_amount: self.get(base + 5), kbd_amount: self.get(base + 6),
            saturation: self.get(base + 7), sem_transition: self.get(base + 10),
            formant_transition: self.get(base + 20), ringmod_amount: self.get(base + 30) }
    }
    pub fn envelope(&self, index: usize) -> EnvelopeModulation {
        assert!(index < 4);
        let base = 600 + index as i32 * 10;
        EnvelopeModulation { attack: self.get(base + 1), decay: self.get(base + 2),
            sustain: self.get(base + 3), release: self.get(base + 4) }
    }
    pub fn lfo(&self, index: usize) -> f32 {
        assert!(index < 4);
        self.get(651 + index as i32 * 10)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModulationField { RowIndex, Source, Scale, Destination1, Destination2 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidModulationId {
    pub row: usize,
    pub field: ModulationField,
    pub id: i32,
}

impl core::fmt::Display for InvalidModulationId {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "invalid modulation {:?} ID {} in row {}", self.field, self.id, self.row)
    }
}
impl core::error::Error for InvalidModulationId {}

#[derive(Clone, Copy, Default)]
struct Route { slot: Option<usize>, global: bool, amount: f32, absolute_amount: f32 }
#[derive(Clone, Copy, Default)]
struct CompiledRow {
    source: ModSource,
    scale: ModSource,
    scale_amount: f32,
    routes: [Route; 2],
}

/// Matrice compilée lors du changement de paramètres, pas dans la boucle audio.
#[derive(Clone, Copy)]
pub struct ModMatrix { rows: [CompiledRow; MOD_MATRIX_ROWS] }
impl Default for ModMatrix {
    fn default() -> Self { Self::new() }
}
impl ModMatrix {
    pub fn new() -> Self { Self { rows: [CompiledRow::default(); MOD_MATRIX_ROWS] } }

    /// Rejette explicitement les IDs inconnus sans modifier la matrice courante.
    /// Aucun fallback silencieux vers une autre destination.
    pub fn configure(&mut self, rows: &[ModMatrixRow; MOD_MATRIX_ROWS]) -> Result<(), InvalidModulationId> {
        let mut compiled = [CompiledRow::default(); MOD_MATRIX_ROWS];
        for (index, row) in rows.iter().enumerate() {
            let source = ModSource::from_id(row.source).ok_or(InvalidModulationId {
                row: index, field: ModulationField::Source, id: row.source })?;
            let scale = ModSource::from_id(row.scale).ok_or(InvalidModulationId {
                row: index, field: ModulationField::Scale, id: row.scale })?;
            let mut routes = [Route::default(); 2];
            for (route, (id, amount, field)) in routes.iter_mut().zip([
                (row.dest1, row.dest1_amount, ModulationField::Destination1),
                (row.dest2, row.dest2_amount, ModulationField::Destination2),
            ]) {
                let destination = ModDest::from_id(id).ok_or(InvalidModulationId { row: index, field, id })?;
                *route = Route { slot: destination_slot(id), global: destination.is_global(),
                    amount, absolute_amount: amount.abs() };
            }
            compiled[index] = CompiledRow { source, scale, scale_amount: row.scale_amount, routes };
        }
        self.rows = compiled;
        Ok(())
    }

    /// Destination polyphonique: sources de cette voix, avec sources globales communes.
    pub fn apply_voice(&self, previous: &ModSources, global: &GlobalSources) -> ModulationValues {
        self.apply(previous, global, false)
    }

    /// Destination globale: sources de la dernière voix, même lorsqu'elle est relâchée.
    /// Appeler une seule fois par échantillon, indépendamment du nombre de voix actives.
    pub fn apply_global(&self, most_recent: &ModSources, global: &GlobalSources) -> ModulationValues {
        self.apply(most_recent, global, true)
    }

    fn apply(&self, sources: &ModSources, global: &GlobalSources, global_destination: bool) -> ModulationValues {
        let mut output = ModulationValues::default();
        for row in &self.rows {
            if row.source == ModSource::None || !row.routes.iter().any(|r| r.slot.is_some() && r.global == global_destination) {
                continue;
            }
            let source = sources.get(row.source, global);
            let scale = if row.scale == ModSource::None { 1.0 }
                else { scale_factor(sources.get(row.scale, global), row.scale_amount) };
            for route in row.routes {
                if route.global == global_destination {
                    if let Some(slot) = route.slot {
                        // Garde l'ordre des multiplications du C++ avant sommation.
                        output.values[slot] += source * route.amount * route.absolute_amount * scale;
                    }
                }
            }
        }
        output
    }
}

/// Facteur scale amont; ni la source bipolaire, ni le facteur ne sont écrêtés.
pub fn scale_factor(source: f32, scale_amount: f32) -> f32 {
    if scale_amount >= 0.0 { 1.0 + (source - 1.0) * scale_amount }
    else { 1.0 + source.abs() * scale_amount }
}

/// Loi de gain asymétrique commune à l'ampli, aux oscillateurs, filtres et master.
/// L'entrée est un gain linéaire (pas des décibels).
pub fn modulate_gain(base_linear: f32, modulation: f32) -> f32 {
    crate::dsp::filters::preset_filter::modulated_gain(base_linear, modulation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_rows() -> [ModMatrixRow; MOD_MATRIX_ROWS] {
        [ModMatrixRow { source: 0, scale: 0, dest1: 0, dest2: 0,
            dest1_amount: 0.0, dest2_amount: 0.0, scale_amount: 0.0 }; MOD_MATRIX_ROWS]
    }

    #[test]
    fn scale_bipolar_and_negative_scale_amount() {
        assert_eq!(scale_factor(-1.0, 1.0), -1.0);
        assert_eq!(scale_factor(-0.5, -1.0), 0.5);
        assert_eq!(scale_factor(0.0, -1.0), 1.0);
    }

    #[test]
    fn both_destinations_and_rows_sum_signed_square() {
        let mut rows = empty_rows();
        rows[0] = ModMatrixRow { source: 202, scale: 405, scale_amount: 1.0,
            dest1: 301, dest2: 901, dest1_amount: 0.5, dest2_amount: -0.5 };
        rows[1] = ModMatrixRow { source: 1000, scale: 0, scale_amount: 0.0,
            dest1: 301, dest2: 0, dest1_amount: -0.25, dest2_amount: 0.0 };
        let mut matrix = ModMatrix::new();
        matrix.configure(&rows).unwrap();
        let local = ModSources { env: [0.0, 0.0, 0.8], velocity: 0.5, ..ModSources::default() };
        let values = matrix.apply_voice(&local, &GlobalSources::default());
        assert!((values.filter(0).freq - 0.0375).abs() < 1e-7);
        assert_eq!(values.get(901), -0.1);
        assert_eq!(values.get(1000), 0.0);
    }

    #[test]
    fn global_reads_most_recent_voice_and_global_scale() {
        let mut rows = empty_rows();
        rows[0] = ModMatrixRow { source: 405, scale: 402, scale_amount: -1.0,
            dest1: 1000, dest2: 4, dest1_amount: 1.0, dest2_amount: 1.0 };
        let mut matrix = ModMatrix::new();
        matrix.configure(&rows).unwrap();
        let global = GlobalSources { modwheel: 0.25, ..GlobalSources::default() };
        let old = ModSources { velocity: 0.2, ..ModSources::default() };
        let recent = ModSources { velocity: 0.8, ..ModSources::default() };
        assert_eq!(matrix.apply_voice(&old, &global).osc(0).vol, 0.15);
        let values = matrix.apply_global(&recent, &global);
        assert_eq!(values.get(1000), 0.6);
        assert_eq!(values.osc(0).vol, 0.0);
    }

    #[test]
    fn unknown_destination_is_reported_without_replacing_matrix() {
        let mut rows = empty_rows();
        rows[0].source = 1000;
        rows[0].dest1 = 1000;
        rows[0].dest1_amount = 1.0;
        let mut matrix = ModMatrix::new();
        matrix.configure(&rows).unwrap();
        rows[0].dest1 = 9999;
        assert_eq!(matrix.configure(&rows), Err(InvalidModulationId {
            row: 0, field: ModulationField::Destination1, id: 9999 }));
        assert_eq!(matrix.apply_global(&ModSources::default(), &GlobalSources::default()).get(1000), 1.0);
    }
}
