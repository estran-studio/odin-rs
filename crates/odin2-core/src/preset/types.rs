//! Paramètres runtime Copy/no_std, unités physiques upstream.
//! Sources: GlobalIncludes.h, AudioValueTree.h, PluginProcessorNonAudioParams.cpp.

/// Identifiants sérialisés upstream (pas des indices contigus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum OscillatorType {
    None = 1,
    #[default]
    Analog = 2,
    Wavetable = 3,
    Multi = 4,
    Vector = 5,
    Chiptune = 6,
    FM = 7,
    PM = 8,
    Noise = 15,
    Wavedraw = 20,
    Chipdraw = 21,
    Specdraw = 22,
}
impl From<i32> for OscillatorType {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::None,
            2 => Self::Analog,
            3 => Self::Wavetable,
            4 => Self::Multi,
            5 => Self::Vector,
            6 => Self::Chiptune,
            7 => Self::FM,
            8 => Self::PM,
            15 => Self::Noise,
            20 => Self::Wavedraw,
            21 => Self::Chipdraw,
            22 => Self::Specdraw,
            _ => Self::None,
        }
    }
}

/// Identifiants sérialisés upstream (pas des indices contigus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AnalogWaveform {
    #[default]
    Saw = 0,
    Pulse = 1,
    Triangle = 2,
    Sine = 3,
}
impl From<i32> for AnalogWaveform {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Saw,
            1 => Self::Pulse,
            2 => Self::Triangle,
            3 => Self::Sine,
            _ => Self::Saw,
        }
    }
}

/// Identifiants sérialisés upstream (pas des indices contigus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FilterType {
    #[default]
    None = 1,
    LP24 = 2,
    LP12 = 3,
    BP24 = 4,
    BP12 = 5,
    HP24 = 6,
    HP12 = 7,
    SEM12 = 15,
    Diode = 19,
    Korg = 24,
    KorgHp = 25,
    Comb = 30,
    Formant = 35,
    RingMod = 40,
}
impl From<i32> for FilterType {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::None,
            2 => Self::LP24,
            3 => Self::LP12,
            4 => Self::BP24,
            5 => Self::BP12,
            6 => Self::HP24,
            7 => Self::HP12,
            15 => Self::SEM12,
            19 => Self::Diode,
            24 => Self::Korg,
            25 => Self::KorgHp,
            30 => Self::Comb,
            35 => Self::Formant,
            40 => Self::RingMod,
            _ => Self::None,
        }
    }
}

/// Identifiants sérialisés upstream (pas des indices contigus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LfoWaveform {
    #[default]
    Sine = 0,
    Saw = 1,
    Triangle = 2,
    Square = 3,
    Square25 = 4,
    Square12 = 5,
    Spike = 6,
    SampleAndHold = 7,
    Pyramid4 = 8,
    Pyramid6 = 9,
    Pyramid8 = 10,
    Pyramid12 = 11,
    Stair3 = 12,
    Stair4 = 13,
    Stair6 = 14,
    Stair8 = 15,
    Stair12 = 16,
    Wavedraw1 = 17,
    Wavedraw2 = 18,
    Wavedraw3 = 19,
    Chipdraw1 = 20,
    Chipdraw2 = 21,
    Chipdraw3 = 22,
}
impl From<i32> for LfoWaveform {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Sine,
            1 => Self::Saw,
            2 => Self::Triangle,
            3 => Self::Square,
            4 => Self::Square25,
            5 => Self::Square12,
            6 => Self::Spike,
            7 => Self::SampleAndHold,
            8 => Self::Pyramid4,
            9 => Self::Pyramid6,
            10 => Self::Pyramid8,
            11 => Self::Pyramid12,
            12 => Self::Stair3,
            13 => Self::Stair4,
            14 => Self::Stair6,
            15 => Self::Stair8,
            16 => Self::Stair12,
            17 => Self::Wavedraw1,
            18 => Self::Wavedraw2,
            19 => Self::Wavedraw3,
            20 => Self::Chipdraw1,
            21 => Self::Chipdraw2,
            22 => Self::Chipdraw3,
            _ => Self::Sine,
        }
    }
}

/// Identifiants sérialisés upstream (pas des indices contigus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DistortionAlgo {
    #[default]
    Clamp = 1,
    Fold = 2,
    Zero = 3,
    Sine = 4,
    Cube = 5,
}
impl From<i32> for DistortionAlgo {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Clamp,
            2 => Self::Fold,
            3 => Self::Zero,
            4 => Self::Sine,
            5 => Self::Cube,
            _ => Self::Clamp,
        }
    }
}

/// Identifiants sérialisés upstream (pas des indices contigus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PlayMode {
    Legato = 0,
    #[default]
    Poly = 1,
    Mono = 2,
}
impl From<i32> for PlayMode {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Legato,
            1 => Self::Poly,
            2 => Self::Mono,
            _ => Self::Poly,
        }
    }
}

/// Identifiants sérialisés upstream (pas des indices contigus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ArpDirection {
    #[default]
    Up = 10,
    Down = 20,
    UpDown = 30,
    DownUp = 40,
    Random = 50,
    CrawlUp = 60,
    CrawlDown = 70,
    CrawlUpDown = 80,
    CrawlDownUp = 90,
}
impl From<i32> for ArpDirection {
    fn from(value: i32) -> Self {
        match value {
            10 => Self::Up,
            20 => Self::Down,
            30 => Self::UpDown,
            40 => Self::DownUp,
            50 => Self::Random,
            60 => Self::CrawlUp,
            70 => Self::CrawlDown,
            80 => Self::CrawlUpDown,
            90 => Self::CrawlDownUp,
            _ => Self::Up,
        }
    }
}

/// Oscillator parameters
#[derive(Debug, Clone, Copy)]
pub struct OscillatorParams {
    pub osc_type: OscillatorType,
    pub analog_wave: AnalogWaveform,
    pub wavetable: i32,
    pub octave: i32,
    pub semitones: i32,
    pub fine: f32,
    pub volume: f32,
    pub reset: bool,
    pub sync: bool,
    pub position: f32,
    pub detune: f32,
    pub pos_mod: f32,
    pub multi_position: f32,
    pub spread: f32,
    pub pulsewidth: f32,
    pub drift: f32,
    pub vec_x: f32,
    pub vec_y: f32,
    pub vec_a: i32,
    pub vec_b: i32,
    pub vec_c: i32,
    pub vec_d: i32,
    pub chipwave: i32,
    pub arp_on: bool,
    pub arp_speed: f32,
    pub step_1: i32,
    pub step_2: i32,
    pub step_3: i32,
    pub step_3_on: bool,
    pub chipnoise: bool,
    pub exp_fm: bool,
    pub fm: f32,
    pub lp: f32,
    pub hp: f32,
    pub modulator_wave: i32,
    pub carrier_wave: i32,
    pub mod_source: i32,
    pub carrier_ratio: i32,
    pub modulator_ratio: i32,
    pub wavedraw: [f32; 193],
    pub chipdraw: [f32; 32],
    pub specdraw: [f32; 49],
}

/// Filter parameters
#[derive(Debug, Clone, Copy)]
pub struct FilterParams {
    pub filter_type: FilterType,
    pub frequency: f32,
    pub resonance: f32,
    pub saturation: f32,
    pub gain: f32,
    pub env_amount: f32,
    pub vel_amount: f32,
    pub kbd_follow: f32,
    pub sem_transition: f32,
    pub formant_transition: f32,
    pub ring_mod_amount: f32,
    pub comb_polarity: bool,
    pub vowel_left: i32,
    pub vowel_right: i32,
    pub osc1_input: bool,
    pub osc2_input: bool,
    pub osc3_input: bool,
    pub fil1_input: bool,
    pub to_amp: bool,
}

/// ADSR envelope parameters
#[derive(Debug, Clone, Copy)]
pub struct EnvelopeParams {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub loop_env: bool,
}

/// LFO parameters
#[derive(Debug, Clone, Copy)]
pub struct LfoParams {
    pub waveform: LfoWaveform,
    pub frequency: f32,
    pub reset: bool,
    pub sync: bool,
    pub sync_numerator: i32,
    pub sync_denominator: i32,
}

/// Delay effect parameters
#[derive(Debug, Clone, Copy)]
pub struct DelayParams {
    pub on: bool,
    pub time: f32,
    pub feedback: f32,
    pub dry: f32,
    pub wet: f32,
    pub hp: f32,
    pub ducking: f32,
    pub pingpong: bool,
    pub sync: bool,
    pub sync_numerator: i32,
    pub sync_denominator: i32,
}

/// Reverb effect parameters
#[derive(Debug, Clone, Copy)]
pub struct ReverbParams {
    pub on: bool,
    pub delay: f32,
    pub dry_wet: f32,
    pub mid_hall: f32,
    pub hf_damp: f32,
    pub eq_gain: f32,
    pub eq_freq: f32,
}

/// Chorus effect parameters
#[derive(Debug, Clone, Copy)]
pub struct ChorusParams {
    pub on: bool,
    pub amount: f32,
    pub rate: f32,
    pub feedback: f32,
    pub dry_wet: f32,
    pub reset: bool,
    pub sync: bool,
    pub sync_numerator: i32,
    pub sync_denominator: i32,
}

/// Phaser effect parameters
#[derive(Debug, Clone, Copy)]
pub struct PhaserParams {
    pub on: bool,
    pub frequency: f32,
    pub feedback: f32,
    pub rate: f32,
    pub mod_amount: f32,
    pub dry_wet: f32,
    pub reset: bool,
    pub sync: bool,
    pub sync_numerator: i32,
    pub sync_denominator: i32,
}

/// Flanger effect parameters
#[derive(Debug, Clone, Copy)]
pub struct FlangerParams {
    pub on: bool,
    pub amount: f32,
    pub rate: f32,
    pub feedback: f32,
    pub dry_wet: f32,
    pub reset: bool,
    pub sync: bool,
    pub sync_numerator: i32,
    pub sync_denominator: i32,
}

/// Distortion effect parameters
#[derive(Debug, Clone, Copy)]
pub struct DistortionParams {
    pub on: bool,
    pub algorithm: DistortionAlgo,
    pub boost: f32,
    pub dry_wet: f32,
}

/// Modulation matrix row
#[derive(Debug, Clone, Copy)]
pub struct ModMatrixRow {
    pub source: i32,
    pub dest1: i32,
    pub dest2: i32,
    pub scale: i32,
    /// Fichier `amount_0_row_N` : montant de la destination 1 (`setModAmount1` amont).
    pub dest1_amount: f32,
    /// Fichier `amount_1_row_N` : montant de la destination 2 (`setModAmount2` amont).
    pub dest2_amount: f32,
    /// Fichier `amount_2_row_N` : montant du facteur d'échelle (`setScaleAmount` amont).
    pub scale_amount: f32,
}

/// Arpeggiator step
#[derive(Debug, Clone, Copy)]
pub struct ArpStep {
    pub on: bool,
    pub transpose: i32,
    pub mod1: f32,
    pub mod2: f32,
}

/// Arpeggiator parameters
#[derive(Debug, Clone, Copy)]
pub struct ArpeggiatorParams {
    pub on: bool,
    pub one_shot: bool,
    pub direction: ArpDirection,
    pub octaves: i32,
    pub steps: i32,
    pub gate: i32,
    pub mod_transpose: f32,
    pub sync_numerator: i32,
    pub sync_denominator: i32,
    pub step_params: [ArpStep; 16],
}

impl Default for OscillatorParams {
    fn default() -> Self {
        Self {
            osc_type: OscillatorType::Analog,
            analog_wave: AnalogWaveform::Saw,
            wavetable: 1,
            octave: 0,
            semitones: 0,
            fine: 0.0,
            volume: 0.0,
            reset: false,
            sync: false,
            position: 0.0,
            detune: 0.2,
            pos_mod: 0.0,
            multi_position: 0.0,
            spread: 0.0,
            pulsewidth: 0.5,
            drift: 0.0,
            vec_x: 0.0,
            vec_y: 0.0,
            vec_a: 101,
            vec_b: 102,
            vec_c: 103,
            vec_d: 104,
            chipwave: 1,
            arp_on: false,
            arp_speed: 20.0,
            step_1: 0,
            step_2: 12,
            step_3: 7,
            step_3_on: false,
            chipnoise: false,
            exp_fm: false,
            fm: 0.0,
            lp: 18000.0,
            hp: 80.0,
            modulator_wave: 1,
            carrier_wave: 1,
            mod_source: 1,
            carrier_ratio: 1,
            modulator_ratio: 1,
            wavedraw: core::array::from_fn(|i| libm::sinf(core::f32::consts::TAU * i as f32 / 193.0) * 0.9),
            chipdraw: core::array::from_fn(|i| if i < 16 { 0.875 } else { -0.875 }),
            specdraw: core::array::from_fn(|i| if i == 0 { 1.0 } else { 0.0 }),
        }
    }
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            filter_type: FilterType::LP24,
            frequency: 20000.0,
            resonance: 0.0,
            saturation: 0.0,
            gain: 0.0,
            env_amount: 0.0,
            vel_amount: 0.0,
            kbd_follow: 0.0,
            sem_transition: -1.0,
            formant_transition: 0.0,
            ring_mod_amount: 1.0,
            comb_polarity: false,
            vowel_left: 0,
            vowel_right: 2,
            osc1_input: true,
            osc2_input: true,
            osc3_input: true,
            fil1_input: false,
            to_amp: false,
        }
    }
}

impl Default for EnvelopeParams {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 1.0,
            sustain: 0.5,
            release: 0.01,
            loop_env: false,
        }
    }
}

impl Default for LfoParams {
    fn default() -> Self {
        Self {
            waveform: LfoWaveform::Sine,
            frequency: 1.0,
            reset: false,
            sync: false,
            sync_numerator: 2,
            sync_denominator: 5,
        }
    }
}

impl Default for DelayParams {
    fn default() -> Self {
        Self {
            on: false,
            time: 1.0,
            feedback: 0.45,
            dry: 1.0,
            wet: 0.7,
            hp: 80.0,
            ducking: 0.0,
            pingpong: false,
            sync: false,
            sync_numerator: 2,
            sync_denominator: 5,
        }
    }
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            on: false,
            delay: 40.0,
            dry_wet: 0.25,
            mid_hall: 2.0,
            hf_damp: 6000.0,
            eq_gain: 0.0,
            eq_freq: 1000.0,
        }
    }
}

impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            on: false,
            amount: 0.4,
            rate: 0.15,
            feedback: 0.0,
            dry_wet: 1.0,
            reset: false,
            sync: false,
            sync_numerator: 2,
            sync_denominator: 5,
        }
    }
}

impl Default for PhaserParams {
    fn default() -> Self {
        Self {
            on: false,
            frequency: 4000.0,
            feedback: 0.25,
            rate: 0.25,
            mod_amount: 0.3,
            dry_wet: 1.0,
            reset: false,
            sync: false,
            sync_numerator: 2,
            sync_denominator: 5,
        }
    }
}

impl Default for FlangerParams {
    fn default() -> Self {
        Self {
            on: false,
            amount: 0.3,
            rate: 0.15,
            feedback: 0.6,
            dry_wet: 1.0,
            reset: false,
            sync: false,
            sync_numerator: 2,
            sync_denominator: 5,
        }
    }
}

impl Default for DistortionParams {
    fn default() -> Self {
        Self {
            on: false,
            algorithm: DistortionAlgo::Clamp,
            boost: 0.3,
            dry_wet: 1.0,
        }
    }
}

impl Default for ModMatrixRow {
    fn default() -> Self {
        Self {
            source: 0,
            dest1: 0,
            dest2: 0,
            scale: 0,
            dest1_amount: 0.0,
            dest2_amount: 0.0,
            scale_amount: 0.0,
        }
    }
}

impl Default for ArpStep {
    fn default() -> Self {
        Self {
            on: true,
            transpose: 0,
            mod1: 0.0,
            mod2: 0.0,
        }
    }
}

impl Default for ArpeggiatorParams {
    fn default() -> Self {
        Self {
            on: false,
            one_shot: false,
            direction: ArpDirection::Up,
            octaves: 2,
            steps: 16,
            gate: 50,
            mod_transpose: 0.0,
            sync_numerator: 1,
            sync_denominator: 5,
            step_params: [ArpStep::default(); 16],
        }
    }
}

/// Catalogue traduit de Source/AudioValueTree.h. Le callback reçoit
/// `(id, chemin structuré, float|int|bool, minimum, maximum, défaut)`.
#[macro_export]
macro_rules! odin_audio_parameters {
    ($callback:ident $(, $arg:tt)*) => {
        $callback! { $($arg)*;
            ("osc1_oct", osc1.octave, int, -4.0, 4.0, 0.0),
            ("osc1_semi", osc1.semitones, int, -12.0, 12.0, 0.0),
            ("osc1_fine", osc1.fine, float, -50.0, 50.0, 0.0),
            ("osc1_vol", osc1.volume, float, -60.0, 12.0, 0.0),
            ("osc1_reset", osc1.reset, bool, 0.0, 1.0, 0.0),
            ("osc1_position", osc1.position, float, 0.0, 1.0, 0.0),
            ("osc1_detune", osc1.detune, float, 0.0, 1.0, 0.2),
            ("osc1_pos_mod", osc1.pos_mod, float, -1.0, 1.0, 0.0),
            ("osc1_multi_position", osc1.multi_position, float, 0.0, 1.0, 0.0),
            ("osc1_spread", osc1.spread, float, 0.0, 1.0, 0.0),
            ("osc1_pulsewidth", osc1.pulsewidth, float, 0.0, 1.0, 0.5),
            ("osc1_drift", osc1.drift, float, 0.0, 1.0, 0.0),
            ("osc1_vec_x", osc1.vec_x, float, 0.0, 1.0, 0.0),
            ("osc1_vec_y", osc1.vec_y, float, 0.0, 1.0, 0.0),
            ("osc1_arp_speed", osc1.arp_speed, float, 1.0, 100.0, 20.0),
            ("osc1_step_1", osc1.step_1, int, -12.0, 12.0, 0.0),
            ("osc1_step_2", osc1.step_2, int, -12.0, 12.0, 12.0),
            ("osc1_step_3", osc1.step_3, int, -12.0, 12.0, 7.0),
            ("osc1_arp_on", osc1.arp_on, bool, 0.0, 1.0, 0.0),
            ("osc1_step_3_on", osc1.step_3_on, bool, 0.0, 1.0, 0.0),
            ("osc1_chipnoise", osc1.chipnoise, bool, 0.0, 1.0, 0.0),
            ("osc1_exp_fm", osc1.exp_fm, bool, 0.0, 1.0, 0.0),
            ("osc1_fm", osc1.fm, float, 0.0, 1.0, 0.0),
            ("osc1_lp", osc1.lp, float, 80.0, 18000.0, 18000.0),
            ("osc1_hp", osc1.hp, float, 80.0, 18000.0, 80.0),
            ("osc2_oct", osc2.octave, int, -4.0, 4.0, 0.0),
            ("osc2_semi", osc2.semitones, int, -12.0, 12.0, 0.0),
            ("osc2_fine", osc2.fine, float, -50.0, 50.0, 0.0),
            ("osc2_vol", osc2.volume, float, -60.0, 12.0, 0.0),
            ("osc2_reset", osc2.reset, bool, 0.0, 1.0, 0.0),
            ("osc2_sync", osc2.sync, bool, 0.0, 1.0, 0.0),
            ("osc2_position", osc2.position, float, 0.0, 1.0, 0.0),
            ("osc2_detune", osc2.detune, float, 0.0, 1.0, 0.2),
            ("osc2_pos_mod", osc2.pos_mod, float, -1.0, 1.0, 0.0),
            ("osc2_multi_position", osc2.multi_position, float, 0.0, 1.0, 0.0),
            ("osc2_spread", osc2.spread, float, 0.0, 1.0, 0.0),
            ("osc2_pulsewidth", osc2.pulsewidth, float, 0.0, 1.0, 0.5),
            ("osc2_drift", osc2.drift, float, 0.0, 1.0, 0.0),
            ("osc2_vec_x", osc2.vec_x, float, 0.0, 1.0, 0.0),
            ("osc2_vec_y", osc2.vec_y, float, 0.0, 1.0, 0.0),
            ("osc2_arp_speed", osc2.arp_speed, float, 1.0, 100.0, 20.0),
            ("osc2_step_1", osc2.step_1, int, -12.0, 12.0, 0.0),
            ("osc2_step_2", osc2.step_2, int, -12.0, 12.0, 12.0),
            ("osc2_step_3", osc2.step_3, int, -12.0, 12.0, 7.0),
            ("osc2_arp_on", osc2.arp_on, bool, 0.0, 1.0, 0.0),
            ("osc2_step_3_on", osc2.step_3_on, bool, 0.0, 1.0, 0.0),
            ("osc2_chipnoise", osc2.chipnoise, bool, 0.0, 1.0, 0.0),
            ("osc2_exp_fm", osc2.exp_fm, bool, 0.0, 1.0, 0.0),
            ("osc2_fm", osc2.fm, float, 0.0, 1.0, 0.0),
            ("osc2_lp", osc2.lp, float, 80.0, 18000.0, 18000.0),
            ("osc2_hp", osc2.hp, float, 80.0, 18000.0, 80.0),
            ("osc3_oct", osc3.octave, int, -4.0, 4.0, 0.0),
            ("osc3_semi", osc3.semitones, int, -12.0, 12.0, 0.0),
            ("osc3_fine", osc3.fine, float, -50.0, 50.0, 0.0),
            ("osc3_vol", osc3.volume, float, -60.0, 12.0, 0.0),
            ("osc3_reset", osc3.reset, bool, 0.0, 1.0, 0.0),
            ("osc3_sync", osc3.sync, bool, 0.0, 1.0, 0.0),
            ("osc3_position", osc3.position, float, 0.0, 1.0, 0.0),
            ("osc3_detune", osc3.detune, float, 0.0, 1.0, 0.2),
            ("osc3_pos_mod", osc3.pos_mod, float, -1.0, 1.0, 0.0),
            ("osc3_multi_position", osc3.multi_position, float, 0.0, 1.0, 0.0),
            ("osc3_spread", osc3.spread, float, 0.0, 1.0, 0.0),
            ("osc3_pulsewidth", osc3.pulsewidth, float, 0.0, 1.0, 0.5),
            ("osc3_drift", osc3.drift, float, 0.0, 1.0, 0.0),
            ("osc3_vec_x", osc3.vec_x, float, 0.0, 1.0, 0.0),
            ("osc3_vec_y", osc3.vec_y, float, 0.0, 1.0, 0.0),
            ("osc3_arp_speed", osc3.arp_speed, float, 1.0, 100.0, 20.0),
            ("osc3_step_1", osc3.step_1, int, -12.0, 12.0, 0.0),
            ("osc3_step_2", osc3.step_2, int, -12.0, 12.0, 12.0),
            ("osc3_step_3", osc3.step_3, int, -12.0, 12.0, 7.0),
            ("osc3_arp_on", osc3.arp_on, bool, 0.0, 1.0, 0.0),
            ("osc3_step_3_on", osc3.step_3_on, bool, 0.0, 1.0, 0.0),
            ("osc3_chipnoise", osc3.chipnoise, bool, 0.0, 1.0, 0.0),
            ("osc3_exp_fm", osc3.exp_fm, bool, 0.0, 1.0, 0.0),
            ("osc3_fm", osc3.fm, float, 0.0, 1.0, 0.0),
            ("osc3_lp", osc3.lp, float, 80.0, 18000.0, 18000.0),
            ("osc3_hp", osc3.hp, float, 80.0, 18000.0, 80.0),
            ("fil1_osc1", filter1.osc1_input, bool, 0.0, 1.0, 1.0),
            ("fil1_osc2", filter1.osc2_input, bool, 0.0, 1.0, 1.0),
            ("fil1_osc3", filter1.osc3_input, bool, 0.0, 1.0, 1.0),
            ("fil1_vel", filter1.vel_amount, float, 0.0, 1.0, 0.0),
            ("fil1_env", filter1.env_amount, float, -1.0, 1.0, 0.0),
            ("fil1_kbd", filter1.kbd_follow, float, 0.0, 1.0, 0.0),
            ("fil1_gain", filter1.gain, float, -60.0, 12.0, 0.0),
            ("fil1_freq", filter1.frequency, float, 20.0, 20000.0, 20000.0),
            ("fil1_res", filter1.resonance, float, 0.0, 1.0, 0.0),
            ("fil1_saturation", filter1.saturation, float, 0.0, 1.0, 0.0),
            ("fil1_sem_transition", filter1.sem_transition, float, -1.0, 1.0, -1.0),
            ("fil1_formant_transition", filter1.formant_transition, float, 0.0, 1.0, 0.0),
            ("fil1_ring_mod_amount", filter1.ring_mod_amount, float, 0.0, 1.0, 1.0),
            ("fil2_osc1", filter2.osc1_input, bool, 0.0, 1.0, 0.0),
            ("fil2_osc2", filter2.osc2_input, bool, 0.0, 1.0, 0.0),
            ("fil2_osc3", filter2.osc3_input, bool, 0.0, 1.0, 0.0),
            ("fil2_fil1", filter2.fil1_input, bool, 0.0, 1.0, 1.0),
            ("fil2_vel", filter2.vel_amount, float, 0.0, 1.0, 0.0),
            ("fil2_env", filter2.env_amount, float, -1.0, 1.0, 0.0),
            ("fil2_kbd", filter2.kbd_follow, float, 0.0, 1.0, 0.0),
            ("fil2_gain", filter2.gain, float, -60.0, 12.0, 0.0),
            ("fil2_freq", filter2.frequency, float, 20.0, 20000.0, 2000.0),
            ("fil2_res", filter2.resonance, float, 0.0, 1.0, 0.0),
            ("fil2_saturation", filter2.saturation, float, 0.0, 1.0, 0.0),
            ("fil2_sem_transition", filter2.sem_transition, float, -1.0, 1.0, -1.0),
            ("fil2_formant_transition", filter2.formant_transition, float, 0.0, 1.0, 0.0),
            ("fil2_ring_mod_amount", filter2.ring_mod_amount, float, 0.0, 1.0, 1.0),
            ("fil3_vel", filter3.vel_amount, float, 0.0, 1.0, 0.0),
            ("fil3_env", filter3.env_amount, float, -1.0, 1.0, 0.0),
            ("fil3_kbd", filter3.kbd_follow, float, 0.0, 1.0, 0.0),
            ("fil3_gain", filter3.gain, float, -60.0, 12.0, 0.0),
            ("fil3_freq", filter3.frequency, float, 20.0, 20000.0, 2000.0),
            ("fil3_res", filter3.resonance, float, 0.0, 1.0, 0.0),
            ("fil3_saturation", filter3.saturation, float, 0.0, 1.0, 0.0),
            ("fil3_sem_transition", filter3.sem_transition, float, -1.0, 1.0, -1.0),
            ("fil3_formant_transition", filter3.formant_transition, float, 0.0, 1.0, 0.0),
            ("fil3_ring_mod_amount", filter3.ring_mod_amount, float, 0.0, 1.0, 1.0),
            ("amp_velocity", amp_velocity, float, 0.0, 1.0, 0.0),
            ("amp_pan", amp_pan, float, -1.0, 1.0, 0.0),
            ("amp_gain", amp_gain, float, -30.0, 12.0, 0.0),
            ("dist_boost", distortion.boost, float, 0.0, 1.0, 0.3),
            ("dist_drywet", distortion.dry_wet, float, 0.0, 1.0, 1.0),
            ("env1_attack", env1.attack, float, 0.001, 10.0, 0.01),
            ("env1_decay", env1.decay, float, 0.001, 10.0, 1.0),
            ("env1_sustain", env1.sustain, float, 0.0, 1.0, 0.5),
            ("env1_release", env1.release, float, 0.001, 10.0, 0.01),
            ("env1_loop", env1.loop_env, bool, 0.0, 1.0, 0.0),
            ("env2_attack", env2.attack, float, 0.001, 10.0, 0.01),
            ("env2_decay", env2.decay, float, 0.001, 10.0, 0.8),
            ("env2_sustain", env2.sustain, float, 0.0, 1.0, 0.0),
            ("env2_release", env2.release, float, 0.001, 10.0, 0.01),
            ("env2_loop", env2.loop_env, bool, 0.0, 1.0, 0.0),
            ("env3_attack", env3.attack, float, 0.001, 10.0, 0.01),
            ("env3_decay", env3.decay, float, 0.001, 10.0, 0.8),
            ("env3_sustain", env3.sustain, float, 0.0, 1.0, 0.0),
            ("env3_release", env3.release, float, 0.001, 10.0, 0.01),
            ("env3_loop", env3.loop_env, bool, 0.0, 1.0, 0.0),
            ("env4_attack", env4.attack, float, 0.001, 10.0, 0.01),
            ("env4_decay", env4.decay, float, 0.001, 10.0, 1.0),
            ("env4_sustain", env4.sustain, float, 0.0, 1.0, 0.5),
            ("env4_release", env4.release, float, 0.001, 10.0, 0.01),
            ("env4_loop", env4.loop_env, bool, 0.0, 1.0, 0.0),
            ("lfo1_reset", lfo1.reset, bool, 0.0, 1.0, 0.0),
            ("lfo1_freq", lfo1.frequency, float, 0.05, 100.0, 1.0),
            ("lfo2_reset", lfo2.reset, bool, 0.0, 1.0, 0.0),
            ("lfo2_freq", lfo2.frequency, float, 0.05, 100.0, 1.0),
            ("lfo3_reset", lfo3.reset, bool, 0.0, 1.0, 0.0),
            ("lfo3_freq", lfo3.frequency, float, 0.05, 100.0, 1.0),
            ("lfo4_reset", lfo4.reset, bool, 0.0, 1.0, 0.0),
            ("lfo4_freq", lfo4.frequency, float, 0.05, 100.0, 1.0),
            ("fil1_to_amp", filter1.to_amp, bool, 0.0, 1.0, 0.0),
            ("fil2_to_amp", filter2.to_amp, bool, 0.0, 1.0, 1.0),
            ("glide", glide, float, 0.0, 1.0, 0.0),
            ("master", master, float, -50.0, 12.0, -7.0),
            ("modwheel", modwheel, float, 0.0, 1.0, 0.0),
            ("pitchbend", pitchbend, float, -1.0, 1.0, 0.0),
            ("xy_x", xy_x, float, 0.0, 1.0, 0.0),
            ("xy_y", xy_y, float, 0.0, 1.0, 0.0),
            ("delay_pingpong", delay.pingpong, bool, 0.0, 1.0, 0.0),
            ("delay_time", delay.time, float, 0.01, 3.0, 1.0),
            ("delay_feedback", delay.feedback, float, 0.0, 1.0, 0.45),
            ("delay_hp", delay.hp, float, 80.0, 18000.0, 80.0),
            ("delay_ducking", delay.ducking, float, 0.0, 1.0, 0.0),
            ("delay_dry", delay.dry, float, 0.0, 1.0, 1.0),
            ("delay_wet", delay.wet, float, 0.0, 1.0, 0.7),
            ("delay_on", delay.on, bool, 0.0, 1.0, 0.0),
            ("rev_delay", reverb.delay, float, 20.0, 80.0, 40.0),
            ("rev_eqgain", reverb.eq_gain, float, -12.0, 12.0, 0.0),
            ("rev_eqfreq", reverb.eq_freq, float, 100.0, 10000.0, 1000.0),
            ("rev_drywet", reverb.dry_wet, float, 0.0, 1.0, 0.25),
            ("rev_mid_hall", reverb.mid_hall, float, 1.0, 8.0, 2.0),
            ("rev_hf_damp", reverb.hf_damp, float, 1500.0, 24000.0, 6000.0),
            ("reverb_on", reverb.on, bool, 0.0, 1.0, 0.0),
            ("phaser_reset", phaser.reset, bool, 0.0, 1.0, 0.0),
            ("phaser_on", phaser.on, bool, 0.0, 1.0, 0.0),
            ("phaser_freq", phaser.frequency, float, 40.0, 8000.0, 4000.0),
            ("phaser_feedback", phaser.feedback, float, 0.0, 1.0, 0.25),
            ("phaser_rate", phaser.rate, float, 0.05, 20.0, 0.25),
            ("phaser_mod", phaser.mod_amount, float, 0.0, 1.0, 0.3),
            ("phaser_drywet", phaser.dry_wet, float, 0.0, 1.0, 1.0),
            ("flanger_reset", flanger.reset, bool, 0.0, 1.0, 0.0),
            ("flanger_on", flanger.on, bool, 0.0, 1.0, 0.0),
            ("flanger_rate", flanger.rate, float, 0.05, 20.0, 0.15),
            ("flanger_feedback", flanger.feedback, float, -1.0, 1.0, 0.6),
            ("flanger_amount", flanger.amount, float, 0.0, 1.0, 0.3),
            ("flanger_drywet", flanger.dry_wet, float, 0.0, 1.0, 1.0),
            ("chorus_reset", chorus.reset, bool, 0.0, 1.0, 0.0),
            ("chorus_on", chorus.on, bool, 0.0, 1.0, 0.0),
            ("chorus_rate", chorus.rate, float, 0.05, 20.0, 0.15),
            ("chorus_amount", chorus.amount, float, 0.0, 1.0, 0.4),
            ("chorus_feedback", chorus.feedback, float, -1.0, 1.0, 0.0),
            ("chorus_drywet", chorus.dry_wet, float, 0.0, 1.0, 1.0),
            ("unison_detune", unison_detune, float, 0.0, 0.6, 0.08),
            ("unison_width", unison_width, float, 0.0, 1.0, 0.7),
            ("step_0_on", arpeggiator.step_params[0].on, bool, 0.0, 1.0, 1.0),
            ("step_1_on", arpeggiator.step_params[1].on, bool, 0.0, 1.0, 1.0),
            ("step_2_on", arpeggiator.step_params[2].on, bool, 0.0, 1.0, 1.0),
            ("step_3_on", arpeggiator.step_params[3].on, bool, 0.0, 1.0, 1.0),
            ("step_4_on", arpeggiator.step_params[4].on, bool, 0.0, 1.0, 1.0),
            ("step_5_on", arpeggiator.step_params[5].on, bool, 0.0, 1.0, 1.0),
            ("step_6_on", arpeggiator.step_params[6].on, bool, 0.0, 1.0, 1.0),
            ("step_7_on", arpeggiator.step_params[7].on, bool, 0.0, 1.0, 1.0),
            ("step_8_on", arpeggiator.step_params[8].on, bool, 0.0, 1.0, 1.0),
            ("step_9_on", arpeggiator.step_params[9].on, bool, 0.0, 1.0, 1.0),
            ("step_10_on", arpeggiator.step_params[10].on, bool, 0.0, 1.0, 1.0),
            ("step_11_on", arpeggiator.step_params[11].on, bool, 0.0, 1.0, 1.0),
            ("step_12_on", arpeggiator.step_params[12].on, bool, 0.0, 1.0, 1.0),
            ("step_13_on", arpeggiator.step_params[13].on, bool, 0.0, 1.0, 1.0),
            ("step_14_on", arpeggiator.step_params[14].on, bool, 0.0, 1.0, 1.0),
            ("step_15_on", arpeggiator.step_params[15].on, bool, 0.0, 1.0, 1.0),
            ("step_0_mod_1", arpeggiator.step_params[0].mod1, float, -1.0, 1.0, 0.0),
            ("step_1_mod_1", arpeggiator.step_params[1].mod1, float, -1.0, 1.0, 0.0),
            ("step_2_mod_1", arpeggiator.step_params[2].mod1, float, -1.0, 1.0, 0.0),
            ("step_3_mod_1", arpeggiator.step_params[3].mod1, float, -1.0, 1.0, 0.0),
            ("step_4_mod_1", arpeggiator.step_params[4].mod1, float, -1.0, 1.0, 0.0),
            ("step_5_mod_1", arpeggiator.step_params[5].mod1, float, -1.0, 1.0, 0.0),
            ("step_6_mod_1", arpeggiator.step_params[6].mod1, float, -1.0, 1.0, 0.0),
            ("step_7_mod_1", arpeggiator.step_params[7].mod1, float, -1.0, 1.0, 0.0),
            ("step_8_mod_1", arpeggiator.step_params[8].mod1, float, -1.0, 1.0, 0.0),
            ("step_9_mod_1", arpeggiator.step_params[9].mod1, float, -1.0, 1.0, 0.0),
            ("step_10_mod_1", arpeggiator.step_params[10].mod1, float, -1.0, 1.0, 0.0),
            ("step_11_mod_1", arpeggiator.step_params[11].mod1, float, -1.0, 1.0, 0.0),
            ("step_12_mod_1", arpeggiator.step_params[12].mod1, float, -1.0, 1.0, 0.0),
            ("step_13_mod_1", arpeggiator.step_params[13].mod1, float, -1.0, 1.0, 0.0),
            ("step_14_mod_1", arpeggiator.step_params[14].mod1, float, -1.0, 1.0, 0.0),
            ("step_15_mod_1", arpeggiator.step_params[15].mod1, float, -1.0, 1.0, 0.0),
            ("step_0_mod_2", arpeggiator.step_params[0].mod2, float, -1.0, 1.0, 0.0),
            ("step_1_mod_2", arpeggiator.step_params[1].mod2, float, -1.0, 1.0, 0.0),
            ("step_2_mod_2", arpeggiator.step_params[2].mod2, float, -1.0, 1.0, 0.0),
            ("step_3_mod_2", arpeggiator.step_params[3].mod2, float, -1.0, 1.0, 0.0),
            ("step_4_mod_2", arpeggiator.step_params[4].mod2, float, -1.0, 1.0, 0.0),
            ("step_5_mod_2", arpeggiator.step_params[5].mod2, float, -1.0, 1.0, 0.0),
            ("step_6_mod_2", arpeggiator.step_params[6].mod2, float, -1.0, 1.0, 0.0),
            ("step_7_mod_2", arpeggiator.step_params[7].mod2, float, -1.0, 1.0, 0.0),
            ("step_8_mod_2", arpeggiator.step_params[8].mod2, float, -1.0, 1.0, 0.0),
            ("step_9_mod_2", arpeggiator.step_params[9].mod2, float, -1.0, 1.0, 0.0),
            ("step_10_mod_2", arpeggiator.step_params[10].mod2, float, -1.0, 1.0, 0.0),
            ("step_11_mod_2", arpeggiator.step_params[11].mod2, float, -1.0, 1.0, 0.0),
            ("step_12_mod_2", arpeggiator.step_params[12].mod2, float, -1.0, 1.0, 0.0),
            ("step_13_mod_2", arpeggiator.step_params[13].mod2, float, -1.0, 1.0, 0.0),
            ("step_14_mod_2", arpeggiator.step_params[14].mod2, float, -1.0, 1.0, 0.0),
            ("step_15_mod_2", arpeggiator.step_params[15].mod2, float, -1.0, 1.0, 0.0),
            ("step_0_transpose", arpeggiator.step_params[0].transpose, int, -24.0, 24.0, 0.0),
            ("step_1_transpose", arpeggiator.step_params[1].transpose, int, -24.0, 24.0, 0.0),
            ("step_2_transpose", arpeggiator.step_params[2].transpose, int, -24.0, 24.0, 0.0),
            ("step_3_transpose", arpeggiator.step_params[3].transpose, int, -24.0, 24.0, 0.0),
            ("step_4_transpose", arpeggiator.step_params[4].transpose, int, -24.0, 24.0, 0.0),
            ("step_5_transpose", arpeggiator.step_params[5].transpose, int, -24.0, 24.0, 0.0),
            ("step_6_transpose", arpeggiator.step_params[6].transpose, int, -24.0, 24.0, 0.0),
            ("step_7_transpose", arpeggiator.step_params[7].transpose, int, -24.0, 24.0, 0.0),
            ("step_8_transpose", arpeggiator.step_params[8].transpose, int, -24.0, 24.0, 0.0),
            ("step_9_transpose", arpeggiator.step_params[9].transpose, int, -24.0, 24.0, 0.0),
            ("step_10_transpose", arpeggiator.step_params[10].transpose, int, -24.0, 24.0, 0.0),
            ("step_11_transpose", arpeggiator.step_params[11].transpose, int, -24.0, 24.0, 0.0),
            ("step_12_transpose", arpeggiator.step_params[12].transpose, int, -24.0, 24.0, 0.0),
            ("step_13_transpose", arpeggiator.step_params[13].transpose, int, -24.0, 24.0, 0.0),
            ("step_14_transpose", arpeggiator.step_params[14].transpose, int, -24.0, 24.0, 0.0),
            ("step_15_transpose", arpeggiator.step_params[15].transpose, int, -24.0, 24.0, 0.0),
            ("arp_mod_transpose", arpeggiator.mod_transpose, float, 0.0, 1.0, 0.0),
            ("arp_on", arpeggiator.on, bool, 0.0, 1.0, 0.0),
            ("arp_one_shot", arpeggiator.one_shot, bool, 0.0, 1.0, 0.0),
        }
    };
}

/// Type et bornes d'un paramètre audio upstream, sans allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioParameterKind { Float, Int, Bool }

#[derive(Debug, Clone, Copy)]
pub struct AudioParameterDescriptor {
    pub id: &'static str,
    pub minimum: f32,
    pub maximum: f32,
    pub default: f32,
    pub kind: AudioParameterKind,
}
macro_rules! parameter_kind {
    (float) => { AudioParameterKind::Float };
    (int) => { AudioParameterKind::Int };
    (bool) => { AudioParameterKind::Bool };
}
macro_rules! descriptors {
    (; $(($id:literal, $($field:ident $( [ $index:literal ] )?).+, $kind:ident, $min:literal, $max:literal, $default:literal),)*) => {
        pub const AUDIO_PARAMETERS: &[AudioParameterDescriptor] = &[
            $(AudioParameterDescriptor { id: $id, minimum: $min, maximum: $max,
                default: $default, kind: parameter_kind!($kind) },)*
        ];
    };
}
crate::odin_audio_parameters!(descriptors);

/// Conversion exacte des indices de dénominateur sauvegardés (ValueChange.cpp).
pub fn sync_denominator(index: i32) -> f32 {
    match index {
        0 => 1.0, 1 => 2.0, 2 => 4.0, 3 => 8.0, 4 => 12.0,
        5 => 16.0, 6 => 24.0, 7 => 32.0, 8 => 48.0, _ => 16.0,
    }
}

/// Durée en rondes, le numérateur sauvegardé étant zéro-based.
pub fn sync_ratio(numerator_index: i32, denominator_index: i32) -> f32 {
    (numerator_index + 1) as f32 / sync_denominator(denominator_index)
}
