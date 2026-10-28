//! Rendu des onze oscillateurs Odin 2, sans allocation dans le chemin audio.
//! Traduction de Source/audio/Oscillators/{Oscillator,WavetableOsc1D,
//! WavetableOsc2D,AnalogOscillator,MultiOscillator,VectorOscillator,
//! ChiptuneOscillator,FMOscillator,PMOscillator,NoiseOscillator}.cpp.
use alloc::sync::Arc;
use crate::constants::{SUBTABLES_PER_WAVETABLE, WAVETABLE_LENGTH};
use crate::preset::{AnalogWaveform, OscillatorParams, OscillatorType};
use super::draw::DrawTableBank;

#[derive(Debug, Default, Clone, Copy)]
pub struct OscillatorModulation {
    pub pitch_semitones: f32,
    /// Entrée exponentielle normalisée: 24 demi-tons par unité.
    pub exponential_fm: f32,
    /// Entrée linéaire normalisée: deux fois la fréquence par unité.
    pub linear_fm: f32,
    pub phase: f32,
    pub position: f32,
    pub pulse_width: f32,
    pub vector_x: f32,
    pub vector_y: f32,
    pub fm_amount: f32,
    pub fm_ratio: f32,
    pub carrier_ratio: f32,
    pub multi_detune: f32,
    pub spread: f32,
    pub arp_speed: f32,
    pub noise_hp: f32,
    pub noise_lp: f32,
    pub drift: f32,
}

/// Les compteurs et le PRNG sont locaux à la voix (résultat déterministe).
#[derive(Clone)]
pub struct PresetOscillator {
    params: OscillatorParams,
    tables: [usize; 4],
    modulator_table: usize,
    modulation: OscillatorModulation,
    draws: Option<Arc<DrawTableBank>>,
    oscillator_index: usize,
    sample_rate: f32,
    inverse_rate: f64,
    frequency: f64,
    tuning: f32,
    phases: [f64; 4],
    multi_output: [f32; 4],
    bands: [BandCache; 3],
    modulator_phase: f64,
    pm_last: f32,
    pm_previous: f32,
    position_smooth: f32,
    vector_smooth: [f32; 2],
    duty_smooth: f32,
    source: f32,
    random: u32,
    drift_counter: u32,
    drift_last: f32,
    drift_next: f32,
    arp_phase: f32,
    arp_step: usize,
    chip_phase: f64,
    chip_value: f32,
    lowpass_memory: f64,
    highpass_memory: f64,
    sync_input: bool,
    sync_position: f64,
    wrapped: bool,
    reset_position: f64,
    down_x: [f64; 10],
    down_y: [f64; 10],
    dc_input: f64,
    dc_output: f64,
}

impl PresetOscillator {
    pub fn new(sample_rate: f32) -> Self {
        let params = OscillatorParams::default();
        let mut oscillator = Self {
            params, modulation: OscillatorModulation::default(), draws: None,
            oscillator_index: 0, tables: [7; 4], modulator_table: 0,
            sample_rate, inverse_rate: 1.0 / sample_rate as f64,
            frequency: 440.0, tuning: 1.0, phases: [0.0; 4], modulator_phase: 0.0,
            multi_output: [0.0; 4], bands: [BandCache::default(); 3],
            pm_last: 0.0, pm_previous: 0.0, position_smooth: 0.0, vector_smooth: [0.0; 2],
            duty_smooth: 0.5, source: 0.0, random: 0x6d2b79f5,
            drift_counter: 0, drift_last: 0.0, drift_next: 0.0,
            arp_phase: 0.0, arp_step: 0, chip_phase: 0.0, chip_value: 0.0,
            lowpass_memory: 0.0, highpass_memory: 0.0, sync_input: false,
            sync_position: 0.0, wrapped: false, reset_position: 0.0,
            down_x: [0.0; 10], down_y: [0.0; 10], dc_input: 0.0, dc_output: 0.0,
        };
        oscillator.drift_last = oscillator.random_bipolar();
        oscillator.drift_next = oscillator.random_bipolar();
        oscillator
    }

    /// Configuration numérique sans allocation; les tables sont préparées séparément.
    pub fn configure(&mut self, params: &OscillatorParams) {
        self.params = *params;
        // AnalogOscillator::setPWMDuty (AnalogOscillator.h:30–35).
        self.params.pulsewidth = params.pulsewidth.clamp(0.02, 0.98);
        // Voice.h:273–316: trois multiplicateurs approchés séparément, puis multipliés
        // (Oscillator.cpp:91–104); pitch(a + b) ≠ pitch(a) * pitch(b) à l'ordre 1e-5.
        self.tuning = pitch((params.octave * 12) as f32) * pitch(params.semitones as f32) * pitch(params.fine * 0.01);
        self.tables = match params.osc_type {
            OscillatorType::Vector => [map_vector(params.vec_a), map_vector(params.vec_b), map_vector(params.vec_c), map_vector(params.vec_d)],
            OscillatorType::FM | OscillatorType::PM => [map_vector(params.carrier_wave); 4],
            OscillatorType::Chiptune => [map_chip(params.chipwave); 4],
            OscillatorType::Wavetable | OscillatorType::Multi => MORPH_TABLES[(params.wavetable - 1).clamp(0, 34) as usize],
            _ => [7; 4],
        };
        self.modulator_table = map_vector(params.modulator_wave);
    }
    pub fn set_draw_tables(&mut self, tables: Arc<DrawTableBank>) { self.draws = Some(tables); }
    /// Indice 0..2 pour sélectionner le dessin propre de cet oscillateur.
    pub fn set_oscillator_index(&mut self, index: usize) { self.oscillator_index = index.min(2); }
    pub(crate) fn set_raw_tables(&mut self, tables: [usize; 4]) { self.tables = tables; }
    pub(crate) fn set_raw_modulator_table(&mut self, table: usize) { self.modulator_table = table; }
    pub(crate) fn multi_outputs(&self) -> [f32; 4] { self.multi_output }
    pub fn set_frequency(&mut self, frequency: f32) { self.frequency = frequency as f64; }
    /// Fréquence de base en double (Oscillator::m_osc_freq_base), après glide et détune d'unisson.
    pub fn set_frequency_f64(&mut self, frequency: f64) { self.frequency = frequency; }
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.inverse_rate = 1.0 / sample_rate as f64;
    }
    pub fn set_modulation(&mut self, modulation: OscillatorModulation) { self.modulation = modulation; }
    pub fn set_position_source(&mut self, source: f32) { self.source = source; }
    pub fn set_sync_input(&mut self, wrapped: bool) { self.sync_input = wrapped; }
    pub fn voice_start(&mut self, randomize_phase: bool) {
        if self.params.reset { self.reset(); }
        else if randomize_phase { self.randomize_phase(); }
        self.arp_phase = 0.0; self.arp_step = 0;
        // setBaseFrequency (WavetableOsc2D.h:23–27, VectorOscillator.h:26–31): le démarrage de
        // note recale les lissages de position sur leur cible.
        self.snap_smoothing();
    }
    fn snap_smoothing(&mut self) {
        self.position_smooth = self.params.position;
        self.vector_smooth = [self.params.vec_x, self.params.vec_y];
    }
    /// Fraction de cycle résiduelle du maître, pour la synchronisation sous-échantillon.
    pub fn set_sync_position(&mut self, position: f32) { self.sync_position = position as f64; }
    pub fn did_wrap(&self) -> bool { self.wrapped }
    pub fn reset_position(&self) -> f32 { self.reset_position as f32 }
    pub fn retrigger(&mut self) { if self.params.reset { self.reset(); } }
    pub fn reset(&mut self) {
        self.phases = [0.0; 4]; self.modulator_phase = 0.0;
        self.snap_smoothing();
        self.arp_phase = 0.0; self.arp_step = 0; self.chip_phase = 0.0; self.chip_value = 0.0;
        self.lowpass_memory = 0.0; self.highpass_memory = 0.0;
        self.down_x = [0.0; 10]; self.down_y = [0.0; 10];
        self.dc_input = 0.0; self.dc_output = 0.0;
        self.sync_input = false; self.wrapped = false; self.reset_position = 0.0;
    }
    pub fn randomize_phase(&mut self) {
        for i in 0..4 { self.phases[i] = self.random_bipolar() as f64 * 0.5 + 0.5; }
        self.modulator_phase = self.random_bipolar() as f64 * 0.5 + 0.5;
    }
    pub fn set_seed(&mut self, seed: u32) { self.random = seed.max(1); }
    fn random_bits(&mut self) -> u32 {
        let mut x = self.random; x ^= x << 13; x ^= x >> 17; x ^= x << 5;
        self.random = x; x
    }
    fn random_bipolar(&mut self) -> f32 { self.random_bits() as f32 / u32::MAX as f32 * 2.0 - 1.0 }

    pub fn process(&mut self) -> f32 {
        self.wrapped = false;
        // m_mod_exp_other: dérive (AnalogOscillator.cpp:45–51) ou demi-tons d'arpège (ChiptuneOscillator.cpp:132–135).
        let mut other = 0.0f32;
        if self.params.osc_type == OscillatorType::Analog {
            let length = (5.0 * self.sample_rate).max(1.0) as u32;
            self.drift_counter += 1;
            if self.drift_counter > length {
                self.drift_counter = 0;
                self.drift_last = self.drift_next;
                self.drift_next = self.random_bipolar();
            }
            let drift = self.drift_last + (self.drift_next - self.drift_last) * self.drift_counter as f32 / length as f32;
            other = drift * (self.params.drift + self.modulation.drift) * 0.2;
        }
        if self.params.osc_type == OscillatorType::Chiptune && self.params.arp_on {
            // ChiptuneArpeggiator.cpp:27–52: position et incrément en float.
            let inverse_rate = (1.0 / self.sample_rate as f64) as f32;
            let mut increment = self.params.arp_speed * inverse_rate;
            if self.modulation.arp_speed != 0.0 {
                increment *= crate::dsp::math::fast_exp(0.05776226504 * (self.modulation.arp_speed * 24.0));
            }
            self.arp_phase += increment;
            while self.arp_phase > 1.0 {
                self.arp_phase -= 1.0;
                self.arp_step += 1;
                if self.arp_step > 2 || (!self.params.step_3_on && self.arp_step > 1) { self.arp_step = 0; }
            }
            other = [self.params.step_1, self.params.step_2, self.params.step_3][self.arp_step] as f32;
        }
        // Oscillator::update (Oscillator.h:35–65): toute la chaîne en double comme l'amont.
        let mut frequency = self.frequency * self.tuning as f64;
        if self.modulation.pitch_semitones + self.modulation.exponential_fm + other != 0.0 {
            let semitones = (self.modulation.pitch_semitones + self.modulation.exponential_fm * 24.0) as f64 + other as f64;
            frequency *= pitch(semitones as f32) as f64;
        }
        frequency += self.modulation.linear_fm as f64 * frequency * 2.0;
        let unclamped = frequency;
        let frequency = frequency.clamp(-20480.0, 20480.0);
        let sync = self.params.sync && matches!(self.params.osc_type,
            OscillatorType::Analog | OscillatorType::Wavetable | OscillatorType::Vector |
            OscillatorType::Chiptune | OscillatorType::Wavedraw | OscillatorType::Chipdraw | OscillatorType::Specdraw);
        if sync && self.sync_input { self.phases[0] = wrap(self.sync_position); }
        self.sync_input = false;
        if sync {
            let mut output = 0.0;
            for _ in 0..3 {
                // m_sync_anti_aliasing_inc_factor = 0.3333333f (WavetableOsc1D.cpp:167).
                let value = self.render(frequency, unclamped, 0.3333333f32 as f64, true);
                output = self.downsample(value);
            }
            // DCBlockingFilter.h:28–37, coefficient amont dépendant du samplerate.
            let pole = if self.sample_rate > 120000.0 { 0.997 }
                else if self.sample_rate > 90000.0 { 0.9965 } else { 0.995 };
            let filtered = output - self.dc_input + pole * self.dc_output;
            self.dc_input = output; self.dc_output = filtered;
            filtered as f32
        } else { self.render(frequency, unclamped, 1.0, false) }
    }

    /// `sync`: AnalogOscillator::doSquare ignore m_sync_anti_aliasing_inc_factor (AnalogOscillator.cpp:87),
    /// la PWM avance donc à pleine vitesse (x3 par sous-échantillon) quand la synchro est active.
    fn render(&mut self, frequency: f64, unclamped: f64, factor: f64, sync: bool) -> f32 {
        let increment = frequency * self.inverse_rate * factor;
        let sub = if matches!(self.params.osc_type, OscillatorType::FM | OscillatorType::PM | OscillatorType::Multi | OscillatorType::Noise | OscillatorType::None) { 0 }
            else { self.bands[0].select(frequency, true, 1.189207) };
        let phase = self.phases[0];
        let value = match self.params.osc_type {
            OscillatorType::None => return 0.0,
            OscillatorType::Analog => {
                if self.params.analog_wave == AnalogWaveform::Pulse {
                    // PWM_SMOOTHIN_FACTOR 0.998f (GlobalIncludes.h:160).
                    self.duty_smooth = libm::fmaf(self.duty_smooth, 0.998, (1.0f32 - 0.998) * self.params.pulsewidth);
                    let duty = self.duty_smooth + self.modulation.pulse_width / 2.0;
                    let a = self.read(7, sub, phase);
                    let b = self.read(7, sub, phase + duty as f64);
                    let d = duty % 1.0;
                    let value = (a - b) * (2.0 * (1.0 - d).max(d)) * 0.3;
                    // doSquare: avance sans facteur de synchro.
                    self.advance(0, if sync { frequency * self.inverse_rate } else { increment });
                    return value;
                } else {
                    let table = match self.params.analog_wave {
                        AnalogWaveform::Saw => 7, AnalogWaveform::Triangle => 6,
                        AnalogWaveform::Sine => 0, AnalogWaveform::Pulse => unreachable!(),
                    };
                    self.read(table, sub, phase)
                }
            }
            OscillatorType::Wavetable => {
                self.position_smooth = (self.position_smooth as f64 + (self.params.position - self.position_smooth) as f64 * 0.001) as f32;
                self.morph(sub, phase, self.position_smooth + self.modulation.position + self.params.pos_mod * self.source)
            }
            OscillatorType::Multi => {
                // MultiOscillator::update (MultiOscillator.cpp:31–74): incréments et fréquences en float.
                let detune = (self.params.detune * self.params.detune + self.modulation.multi_detune).max(0.0);
                let spread = self.params.spread + self.modulation.spread;
                let mut frequencies = [0.0f32; 4];
                let mut increments = [0.0f64; 4];
                for i in 0..4 {
                    let ratio = cheap_pitch([0.97, -0.348, 0.238, -1.0][i] * detune);
                    frequencies[i] = ((unclamped * ratio as f64) as f32).clamp(-20480.0, 20480.0);
                    increments[i] = (frequencies[i] as f64 * self.inverse_rate * WAVETABLE_LENGTH as f64) as f32 as f64
                        / WAVETABLE_LENGTH as f64;
                }
                let sub = self.bands[2].select(frequencies[0] as f64, false, 1.1892);
                let mut output = 0.0;
                for i in 0..4 {
                    let position = self.params.multi_position + (i as f32 - 1.5) * spread * 0.5 + self.modulation.position;
                    self.multi_output[i] = self.morph(sub, self.phases[i], position);
                    output += self.multi_output[i];
                    self.advance(i, increments[i]);
                }
                return output * 0.25;
            }
            OscillatorType::Vector => {
                self.vector_smooth[0] = (self.vector_smooth[0] as f64 + (self.params.vec_x - self.vector_smooth[0]) as f64 * 0.001) as f32;
                self.vector_smooth[1] = (self.vector_smooth[1] as f64 + (self.params.vec_y - self.vector_smooth[1]) as f64 * 0.001) as f32;
                let x = (self.vector_smooth[0] + self.modulation.vector_x).clamp(0.0, 1.0);
                let y = (self.vector_smooth[1] + self.modulation.vector_y).clamp(0.0, 1.0);
                let a = self.read(self.tables[0], sub, phase);
                let b = self.read(self.tables[1], sub, phase);
                let c = self.read(self.tables[2], sub, phase);
                let d = self.read(self.tables[3], sub, phase);
                // VectorOscillator.cpp:83–90
                let lower = (1.0 - x) * a + x * d;
                let upper = (1.0 - x) * b + x * c;
                (1.0 - y) * lower + y * upper
            }
            OscillatorType::Chiptune => {
                if self.params.chipnoise {
                    self.chip_phase += frequency * self.inverse_rate * WAVETABLE_LENGTH as f64;
                    if self.chip_phase > 90.0 {
                        self.chip_phase = 0.0;
                        self.chip_value = ((self.random_bits() % 16) as i32 - 8) as f32 * 0.125;
                    }
                    let mut output = 0.0;
                    for _ in 0..3 { output = self.downsample(self.chip_value); }
                    return output as f32;
                }
                self.read(self.tables[0], sub, phase)
            }
            OscillatorType::FM | OscillatorType::PM => {
                // FMOscillator.cpp:39–73 / PMOscillator.cpp:39–64.
                let mut mod_frequency = frequency * self.params.modulator_ratio.max(1) as f64 / self.params.carrier_ratio.max(1) as f64;
                if self.modulation.carrier_ratio != 0.0 || self.modulation.fm_ratio != 0.0 {
                    mod_frequency *= pitch((self.modulation.fm_ratio - self.modulation.carrier_ratio) * 24.0) as f64;
                }
                let mod_frequency = mod_frequency.clamp(-20480.0, 20480.0);
                let modulator_sub = self.bands[1].select(mod_frequency, true, 1.189207);
                let modulation = self.read(self.modulator_table, modulator_sub, self.modulator_phase);
                self.modulator_phase = wrap(self.modulator_phase + mod_frequency * self.inverse_rate);
                let amount = self.params.fm + self.modulation.fm_amount;
                if self.params.osc_type == OscillatorType::PM {
                    let pm = amount * modulation + self.modulation.phase;
                    let velocity = pm - self.pm_previous;
                    let carrier_sub = pm_subtable(frequency + velocity as f64 * self.sample_rate as f64);
                    self.pm_previous = self.pm_last; self.pm_last = pm;
                    let output = self.read_pm_carrier(self.tables[0], carrier_sub, phase, pm);
                    self.advance(0, frequency * self.inverse_rate);
                    return output;
                }
                let amount = amount.max(0.0);
                let mut carrier_frequency = frequency;
                if self.params.exp_fm {
                    let semitones = modulation * amount * 24.0;
                    if semitones != 0.0 { carrier_frequency *= pitch(semitones) as f64; }
                } else {
                    carrier_frequency += (modulation * 15.0) as f64 * frequency * amount as f64;
                }
                let carrier_frequency = carrier_frequency.clamp(-20480.0, 20480.0);
                let carrier_sub = self.bands[0].select(carrier_frequency, true, 1.189207);
                let output = self.read(self.tables[0], carrier_sub, phase);
                self.advance(0, carrier_frequency * self.inverse_rate);
                return output;
            }
            OscillatorType::Noise => {
                let white = self.random_bipolar() as f64;
                let low = (self.params.lp * pitch(self.modulation.noise_lp * 64.0)).clamp(20.0, 20000.0)
                    .min(self.sample_rate * 0.49);
                let high = (self.params.hp * pitch(self.modulation.noise_hp * 64.0)).clamp(20.0, 20000.0)
                    .min(self.sample_rate * 0.49);
                let low = one_pole(white, low as f64, self.inverse_rate, &mut self.lowpass_memory);
                return (low - one_pole(low, high as f64, self.inverse_rate, &mut self.highpass_memory)) as f32;
            }
            OscillatorType::Wavedraw | OscillatorType::Chipdraw | OscillatorType::Specdraw => {
                let offset = match self.params.osc_type { OscillatorType::Wavedraw => 0, OscillatorType::Chipdraw => 3, _ => 6 };
                self.read(160 + offset + self.oscillator_index, sub, phase)
            }
        };
        self.advance(0, increment);
        value
    }
    fn advance(&mut self, index: usize, increment: f64) {
        let position = self.phases[index] + increment;
        self.wrapped = position >= 1.0;
        if self.wrapped { self.reset_position = wrap(position); }
        self.phases[index] = wrap(position);
    }
    fn read(&self, table: usize, sub: usize, phase: f64) -> f32 {
        if table < 160 { read_table(crate::wavetables::get_subtable(table, sub), phase) }
        else {
            read_table(self.draws.as_ref().expect("Oscillateur dessin: installer DrawTableBank avant rendu").subtable(table - 160, sub), phase)
        }
    }
    /// PMCarrierOsc::doOscillate (PMCarrierOsc.h:66–86). Les boucles de repliement amont finissent par
    /// `while (index < WAVETABLE_LENGTH) index += WAVETABLE_LENGTH`: l'index lu est donc dans
    /// [512, 1024), soit la sous-table SUIVANTE du tableau contigu `wavetable_data[160][33][512]`
    /// (la sous-table 0 du wavetable suivant après la 32). La fraction est calculée avec une
    /// troncature vers zéro (négative pour une phase négative). Reproduit à l'identique.
    fn read_pm_carrier(&self, table: usize, sub: usize, phase: f64, phase_mod: f32) -> f32 {
        let length = WAVETABLE_LENGTH as i32;
        let x = phase * WAVETABLE_LENGTH as f64 + (phase_mod * WAVETABLE_LENGTH as f32) as f64;
        let truncated = x as i32;
        let fractional = (x - (truncated as f32) as f64) as f32;
        let table = self.following_subtable(table, sub);
        lerp(table[truncated.rem_euclid(length) as usize], table[(truncated + 1).rem_euclid(length) as usize], fractional)
    }
    /// Mémoire qui suit `(table, sub)` dans les tableaux contigus amont; sans suivant, la même sous-table.
    fn following_subtable(&self, table: usize, sub: usize) -> &[f32; WAVETABLE_LENGTH] {
        if table < 160 {
            if sub + 1 < SUBTABLES_PER_WAVETABLE { crate::wavetables::get_subtable(table, sub + 1) }
            else if table + 1 < 160 { crate::wavetables::get_subtable(table + 1, 0) }
            else { crate::wavetables::get_subtable(table, sub) }
        } else {
            let bank = self.draws.as_ref().expect("Oscillateur dessin: installer DrawTableBank avant rendu");
            let draw = table - 160;
            if sub + 1 < SUBTABLES_PER_WAVETABLE { bank.subtable(draw, sub + 1) }
            else if draw % 3 != 2 { bank.subtable(draw + 1, 0) }
            else { bank.subtable(draw, sub) }
        }
    }
    fn morph(&self, sub: usize, phase: f64, position: f32) -> f32 {
        // getTableIndicesAndInterpolation (WavetableOsc2D.h:95–114), seuils amont.
        let position = position.clamp(0.0, 1.0);
        let (left, interpolation) = if position < 0.333333333333f32 { (0, position * 3.0) }
            else if position < 0.666666666f32 { (1, (position - 0.333333333f32) * 3.0) }
            else { (2, (position - 0.6666666f32) * 3.0) };
        let a = self.read(self.tables[left], sub, phase);
        let b = self.read(self.tables[left + 1], sub, phase);
        (1.0 - interpolation) * a + interpolation * b
    }
    fn downsample(&mut self, input: f32) -> f64 {
        self.down_x.copy_within(1..10, 0); self.down_y.copy_within(1..10, 0);
        let x = &mut self.down_x; let y = &mut self.down_y;
        x[9] = input as f64 * 0.019966841051093;
        y[9] = x[0] + x[9] + 9.0 * (x[1] + x[8]) + 36.0 * (x[2] + x[7])
            + 84.0 * (x[3] + x[6]) + 126.0 * (x[4] + x[5])
            - 0.0003977153 * y[0] - 0.0064474617 * y[1] - 0.0476997403 * y[2]
            - 0.2185829743 * y[3] - 0.6649234123 * y[4] - 1.4773657709 * y[5]
            - 2.2721421641 * y[6] - 2.6598673212 * y[7] - 1.8755960587 * y[8];
        y[9]
    }
}

/// Oscillator::pitchShiftMultiplier (Oscillator.cpp:25–32). `0.05776226504` est un littéral double:
/// l'argument de `FastMathApproximations::exp` est donc un double, le Padé est évalué en double et
/// seul le résultat est tronqué en float (1 ulp float = 6e-8 de fréquence, soit 3e-3 d'erreur en 1 s).
pub(crate) fn pitch(semitones: f32) -> f32 {
    let x = 0.05776226504f64 * semitones as f64;
    if semitones < 48.0 && semitones > -48.0 {
        let numerator = 1680.0 + x * (840.0 + x * (180.0 + x * (20.0 + x)));
        let denominator = 1680.0 + x * (-840.0 + x * (180.0 + x * (-20.0 + x)));
        (numerator / denominator) as f32
    } else {
        libm::exp(x) as f32
    }
}
/// MultiOscillator::cheapPitchShiftMultiplier (MultiOscillator.cpp:154–165): `0.04166666` est un
/// littéral double, le polynôme de Horner est évalué en double.
fn cheap_pitch(semitones: f32) -> f32 {
    let x = (semitones * 0.057762265f32) as f64;
    (1.0 + x * (1.0 + x * (0.5 + x * (0.1666666f32 as f64 + x * 0.04166666)))) as f32
}
pub(crate) fn wrap(phase: f64) -> f64 { phase - libm::floor(phase) }
pub(crate) fn lerp(a: f32, b: f32, amount: f32) -> f32 { a + amount * (b - a) }
pub(crate) fn read_table(table: &[f32; WAVETABLE_LENGTH], phase: f64) -> f32 {
    let position = wrap(phase) * WAVETABLE_LENGTH as f64;
    let left = position as usize;
    lerp(table[left], table[(left + 1) % WAVETABLE_LENGTH], (position - left as f64) as f32)
}
#[derive(Clone, Copy, Default)]
pub(crate) struct BandCache { inverse_mid: f32, last: usize }
const INVERSE_MID: [f32; SUBTABLES_PER_WAVETABLE] = [
    0.0396548, 0.0333456, 0.0280402, 0.0235789, 0.0198274, 0.0166728, 0.0140201,
    0.0117895, 0.00991372, 0.00833641, 0.00701006, 0.00589473, 0.00495686, 0.00416821,
    0.00350503, 0.00294737, 0.00247843, 0.0020841, 0.00175252, 0.00147369, 0.00123922,
    0.00104205, 0.000876259, 0.000736843, 0.000619609, 0.000521027, 0.00043813, 0.000368422,
    0.000309804, 0.000260514, 0.000219065, 0.000184211, 0.000154902,
];
impl BandCache {
    /// Sélection avec hystérésis de WavetableOsc1D::getTableIndex / Multi::getTableIndex.
    /// `absolute`: WavetableOsc1D (fréquence double, comparaison en double); sinon Multi (tout en float).
    pub(crate) fn select(&mut self, frequency: f64, absolute: bool, step: f32) -> usize {
        let ratio = if absolute { self.inverse_mid as f64 * frequency } else { (self.inverse_mid * frequency as f32) as f64 };
        if ratio < 1.09050f32 as f64 && ratio > 0.917004f32 as f64 { return self.last; }
        let frequency = if absolute { frequency.abs() as f32 } else { frequency as f32 };
        let mut seed = 27.5f32;
        for (table, &inverse_mid) in INVERSE_MID.iter().enumerate().take(SUBTABLES_PER_WAVETABLE) {
            if frequency < seed { self.inverse_mid = inverse_mid; self.last = table; return table; }
            seed *= step;
        }
        SUBTABLES_PER_WAVETABLE - 1
    }
}
pub(crate) fn subtable_index(frequency: f32) -> usize {
    BandCache::default().select(frequency as f64, true, 1.189207)
}
/// PMCarrierOsc::getTableIndex, sans hystérésis.
fn pm_subtable(frequency: f64) -> usize {
    let mut seed = 27.5f64;
    for sub in 0..SUBTABLES_PER_WAVETABLE {
        if (frequency.abs() as f32 as f64) < seed { return sub; }
        seed *= 1.1892071150;
    }
    SUBTABLES_PER_WAVETABLE - 1
}
fn one_pole(input: f64, frequency: f64, inverse_rate: f64, memory: &mut f64) -> f64 {
    let g = libm::tan(core::f64::consts::PI * frequency * inverse_rate);
    let v = (input - *memory) * g / (1.0 + g);
    let output = v + *memory; *memory = v + output; output
}

/// WavetableMappings.h:50–227 (les identifiants menu ne sont pas des indices).
pub fn map_vector(input: i32) -> usize {
    if input == 1 { return 0; }
    if (601..=603).contains(&input) { return 160 + (input - 601) as usize; }
    if (701..=703).contains(&input) { return 163 + (input - 701) as usize; }
    if (801..=803).contains(&input) { return 166 + (input - 801) as usize; }
    if (501..=506).contains(&input) { return [2, 3, 4, 1, 85, 86][(input - 501) as usize]; }
    let group = input / 10;
    let corner = input % 10;
    if !(1..=4).contains(&corner) { return 0; }
    let tables = match group {
        10 => [5,2,6,0], 12 => [140,141,142,143], 13 => [152,153,154,155],
        14 => [88,89,90,91], 15 => [136,137,138,139], 16 => [44,45,46,47],
        17 => [48,49,50,51], 18 => [148,149,150,151], 19 => [56,57,58,59],
        20 => [144,145,146,147], 21 => [60,61,62,63], 22 => [65,66,67,64],
        23 => [40,41,42,43], 24 => [68,69,70,71], 25 => [76,77,78,79],
        26 => [72,73,74,75], 27 => [108,109,110,111], 28 => [112,113,114,115],
        29 => [116,117,118,119], 30 => [120,121,122,123], 31 => [124,125,126,127],
        32 => [128,129,130,131], 33 => [132,133,134,135], 34 => [8,9,10,11],
        35 => [12,13,14,15], 36 => [17,18,19,16], 37 => [20,21,22,23],
        38 => [32,33,34,35], 39 => [36,37,38,39], 40 => [28,29,30,31],
        41 => [24,25,26,27], 42 => [92,93,94,95], 43 => [96,97,98,99],
        44 => [100,101,102,103], 45 => [104,105,106,107], _ => return 0,
    };
    tables[(corner - 1) as usize]
}
pub fn map_chip(input: i32) -> usize {
    if (1..=12).contains(&input) { [2,3,4,1,85,86,156,81,82,157,84,87][(input - 1) as usize] }
    else if (701..=703).contains(&input) { 163 + (input - 701) as usize }
    else { 0 }
}
/// WavetableOsc2D.cpp:25–201, indices WT_NR originaux.
pub const MORPH_TABLES: [[usize; 4]; 35] = [
    [5,2,6,0], [140,141,142,143], [152,153,154,155], [88,89,90,91],
    [136,137,138,139], [44,45,46,47], [48,49,50,51], [148,149,150,151],
    [56,57,58,59], [144,145,146,147], [60,61,62,63], [65,66,67,64],
    [40,41,42,43], [68,69,70,71], [76,77,78,79], [72,73,74,75],
    [108,109,110,111], [112,113,114,115], [116,117,118,119], [120,121,122,123],
    [124,125,126,127], [128,129,130,131], [132,133,134,135], [8,9,10,11],
    [12,13,14,15], [17,18,19,16], [20,21,22,23], [32,33,34,35],
    [36,37,38,39], [28,29,30,31], [24,25,26,27], [92,93,94,95],
    [96,97,98,99], [100,101,102,103], [104,105,106,107],
];
