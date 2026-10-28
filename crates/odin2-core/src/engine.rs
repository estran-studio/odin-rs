//! Moteur polyphonique : ordre de traitement de PluginProcessorProcess.cpp.

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use crate::arpeggiator::Arpeggiator;
use crate::config::EngineConfig;
use crate::constants::VOICES;
use crate::dsp::effects::EffectsChain;
use crate::dsp::envelopes::{Adsr, Envelope};
use crate::dsp::filters::PresetFilter;
use crate::dsp::oscillators::{DrawTableBank, Lfo};
use crate::mod_matrix::{modulate_gain, GlobalSources, InvalidModulationId, ModMatrix, ModSources, ModulationValues};
use crate::parameters::{ParameterError, ParameterId};
use crate::preset::PlayMode;
use crate::voice::Voice;
use crate::voice_manager::{unison, unison_count, VoiceManager};

/// Interface audio/MIDI, sans allocation dans ses méthodes de rendu et de note.
pub trait SynthEngine {
    fn note_on(&mut self, note: u8, velocity: u8);
    fn note_off(&mut self, note: u8);
    fn control_change(&mut self, cc: u8, value: u8);
    /// Déviation signée, -8192 à 8191, centre zéro.
    fn pitch_bend(&mut self, value: i16);
    /// Sortie entrelacée; canaux >2 laissés à zéro, mono = canal gauche upstream.
    fn process(&mut self, output: &mut [f32], channels: usize);
    /// Reconfiguration hors callback : certains buffers DSP sont réalloués.
    fn set_sample_rate(&mut self, sample_rate: f32);
}

pub struct OdinEngine {
    voices: Box<[Voice]>,
    pub mod_matrix: ModMatrix,
    manager: VoiceManager,
    sample_rate: f32,
    /// Cible linéaire du gain maître, lissée avant la modulation.
    pub master_volume: f32,
    master_smooth: f32,
    config: EngineConfig,
    filter3: [PresetFilter; 2],
    effects: EffectsChain,
    global_env: Adsr,
    global_lfo: Lfo,
    globals: GlobalSources,
    previous_global_mod: ModulationValues,
    xy_smooth: [f32; 2],
    last_note: Option<u8>,
    held_velocity: [u8; 128],
    held_order: [u8; 128],
    held_count: usize,
    arpeggiator: Arpeggiator,
    voice_unison: [(f32, f32, f32); VOICES],
}

impl OdinEngine {
    /// Construit tous les états DSP. À appeler hors thread audio.
    pub fn new(sample_rate: f32) -> Self {
        let config = EngineConfig::default();
        let voice_config = config.voice();
        let mut voices = Vec::with_capacity(VOICES);
        for index in 0..VOICES {
            let mut voice = Voice::new(sample_rate);
            voice.set_seed(0x4f64_696e_u32.wrapping_add(index as u32 * 7919));
            voice.configure(&voice_config);
            voices.push(voice);
        }
        let mut engine = Self {
            voices: voices.into_boxed_slice(), mod_matrix: ModMatrix::new(),
            manager: VoiceManager::new(), sample_rate,
            master_volume: db_to_gain(config.master), master_smooth: 1.0,
            filter3: core::array::from_fn(|_| PresetFilter::new(sample_rate)),
            effects: EffectsChain::new(sample_rate), global_env: Adsr::new(sample_rate),
            global_lfo: Lfo::new(sample_rate), globals: GlobalSources::default(),
            previous_global_mod: ModulationValues::default(), xy_smooth: [0.0; 2],
            last_note: None, held_velocity: [0; 128], held_order: [0; 128], held_count: 0,
            arpeggiator: Arpeggiator::new(), voice_unison: [(0.0, 0.0, 1.0); VOICES], config,
        };
        engine.apply_parameters();
        engine
    }

    pub fn config(&self) -> &EngineConfig { &self.config }

    /// Charge un patch complet hors callback : prépare et partage les tables dessinées.
    /// Une matrice invalide est rejetée avant toute modification de l'état courant.
    pub fn load_config(&mut self, config: EngineConfig) -> Result<(), InvalidModulationId> {
        let mut matrix = ModMatrix::new();
        matrix.configure(&config.mod_matrix)?;
        let tables = Rc::new(DrawTableBank::new(&[config.osc1, config.osc2, config.osc3]));
        self.all_sound_off();
        self.config = config;
        self.mod_matrix = matrix;
        for voice in &mut self.voices { voice.set_draw_tables(Rc::clone(&tables)); }
        self.global_lfo.set_draw_tables(tables);
        self.apply_parameters();
        Ok(())
    }

    #[cfg(feature = "std")]
    pub fn load_preset(&mut self, preset: &crate::preset::OdinPreset) -> Result<(), InvalidModulationId> {
        self.load_config(EngineConfig::from_preset(preset))
    }

    /// Mise à jour d'un paramètre audio sur les voix actives et futures, sans allocation.
    /// Ne recharge ni les dessins ni la matrice, ne réinitialise aucune enveloppe.
    pub fn set_parameter(&mut self, id: ParameterId, value: f32) -> Result<(), ParameterError> {
        id.set(&mut self.config, value)?;
        self.apply_parameters();
        Ok(())
    }

    /// Mise à jour groupée : validation avant mutation, puis un seul recalcul DSP.
    pub fn set_parameters(&mut self, values: &[(ParameterId, f32)]) -> Result<(), ParameterError> {
        if values.iter().any(|(_, value)| !value.is_finite()) {
            return Err(ParameterError::NonFinite);
        }
        for &(id, value) in values { id.set(&mut self.config, value)?; }
        self.apply_parameters();
        Ok(())
    }

    pub fn parameter(&self, id: ParameterId) -> f32 { id.get(&self.config) }

    fn apply_parameters(&mut self) {
        let config = self.config.voice();
        for (index, voice) in self.voices.iter_mut().enumerate() {
            voice.configure(&config);
            voice.set_tempo(self.config.bpm);
            let (pan, detune, gain) = self.voice_unison[index];
            voice.set_unison(pan * self.config.unison_width, detune * self.config.unison_detune, gain);
            voice.set_unison_position(pan);
        }
        for filter in &mut self.filter3 { filter.configure(&self.config.filter3); }
        self.global_env.configure(&self.config.env4);
        self.global_lfo.configure(&self.config.lfo4);
        self.global_lfo.set_tempo(self.config.bpm);
        self.effects.configure(&self.config.delay, &self.config.phaser, &self.config.flanger,
            &self.config.chorus, &self.config.reverb, self.config.fx_positions);
        self.effects.set_bpm(self.config.bpm);
        self.master_volume = db_to_gain(self.config.master);
    }

    /// Tempo hôte en BPM; préserve la phase des LFO et les queues d'effets.
    pub fn set_tempo(&mut self, bpm: f32) {
        if bpm.is_finite() && bpm > 0.0 {
            self.config.bpm = bpm;
            self.apply_parameters();
        }
    }

    pub fn channel_pressure(&mut self, value: u8) { self.globals.channel_pressure = value.min(127) as f32 / 127.0; }
    pub fn poly_aftertouch(&mut self, note: u8, value: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.note == note { voice.set_aftertouch(value.min(127) as f32 / 127.0); }
        }
    }

    pub fn set_modulation_row(&mut self, index: usize, row: crate::preset::ModMatrixRow) -> Result<(), InvalidModulationId> {
        // Validation transactionnelle : une erreur ne remplace pas la matrice active.
        let mut rows = self.config.mod_matrix;
        if let Some(target) = rows.get_mut(index) { *target = row; }
        else {
            return Err(InvalidModulationId {
                row: index, field: crate::mod_matrix::ModulationField::RowIndex, id: index as i32,
            });
        }
        let mut matrix = ModMatrix::new();
        matrix.configure(&rows)?;
        self.mod_matrix = matrix;
        self.config.mod_matrix = rows;
        Ok(())
    }

    fn held_on(&mut self, note: u8, velocity: u8) {
        self.held_off(note);
        self.held_order[self.held_count] = note;
        self.held_count += 1;
        self.held_velocity[note as usize] = velocity;
    }
    fn held_off(&mut self, note: u8) -> bool {
        let newest = self.held_count > 0 && self.held_order[self.held_count - 1] == note;
        if let Some(index) = self.held_order[..self.held_count].iter().position(|&n| n == note) {
            self.held_order.copy_within(index + 1..self.held_count, index);
            self.held_count -= 1;
        }
        self.held_velocity[note as usize] = 0;
        newest
    }

    fn start_note(&mut self, note: u8, velocity: u8, arp: [f32; 2], remember: bool) {
        if remember { self.held_on(note, velocity); }
        if self.config.play_mode == PlayMode::Mono { self.global_env.trigger(); }
        else { self.global_env.trigger_legato(); }
        self.global_lfo.retrigger();
        self.effects.note_on();
        let count = unison_count(self.config.unison_voices);
        let active = core::array::from_fn(|i| self.voices[i].active);
        let releasing = core::array::from_fn(|i| self.voices[i].is_releasing());
        let indices = self.manager.allocate(note, count, self.config.play_mode == PlayMode::Poly, &active, &releasing);
        for (i, &index) in indices[..count].iter().enumerate() {
            let (pan, detune, gain) = unison(count, i);
            self.voice_unison[index] = (pan, detune, gain);
            self.voices[index].set_arp_modulation(arp);
            self.voices[index].start(note, velocity, self.last_note,
                pan * self.config.unison_width, detune * self.config.unison_detune,
                gain, self.config.play_mode == PlayMode::Legato);
            self.voices[index].set_unison_position(pan);
        }
        self.last_note = Some(note);
        for filter in &mut self.filter3 { filter.set_midi_unshifted(note, velocity); }
    }

    fn stop_note(&mut self, note: u8, remember: bool) {
        let newest = remember && self.held_off(note);
        if self.config.play_mode != PlayMode::Poly && newest && self.held_count > 0 {
            let previous = self.held_order[self.held_count - 1];
            self.start_note(previous, self.held_velocity[previous as usize], [0.0; 2], false);
        }
        for (index, voice) in self.voices.iter_mut().enumerate() {
            if voice.active && voice.note == note {
                if self.manager.sustain { self.manager.defer(index, note); }
                else { voice.note_off(); }
            }
        }
        if !self.voices.iter().any(|voice| voice.active && !voice.is_releasing()) { self.global_env.release(); }
    }

    pub fn all_notes_off(&mut self) {
        for voice in &mut self.voices { voice.note_off(); }
        self.manager.deferred.fill(false);
        self.held_count = 0;
        self.held_velocity.fill(0);
        self.arpeggiator.reset();
        self.global_env.release();
    }

    pub fn all_sound_off(&mut self) {
        for voice in &mut self.voices { voice.reset(); }
        for filter in &mut self.filter3 { filter.reset(); }
        self.effects.reset();
        self.global_env.reset();
        self.global_lfo.reset();
        self.manager = VoiceManager::new();
        self.held_count = 0;
        self.held_velocity.fill(0);
        self.arpeggiator.reset();
        self.last_note = None;
        self.previous_global_mod = ModulationValues::default();
    }

    pub fn active_voice_count(&self) -> usize { self.voices.iter().filter(|voice| voice.active).count() }
}

impl SynthEngine for OdinEngine {
    fn note_on(&mut self, note: u8, velocity: u8) {
        if note >= 128 { return; }
        if velocity == 0 { self.note_off(note); return; }
        if self.config.arpeggiator.on { self.arpeggiator.note_on(note, velocity.min(127), &self.config.arpeggiator); }
        else { self.start_note(note, velocity.min(127), [0.0; 2], true); }
    }

    fn note_off(&mut self, note: u8) {
        if note >= 128 { return; }
        if self.config.arpeggiator.on { self.arpeggiator.note_off(note, &self.config.arpeggiator); }
        else { self.stop_note(note, true); }
    }

    fn control_change(&mut self, cc: u8, value: u8) {
        let normalized = value.min(127) as f32 / 127.0;
        match cc {
            1 => self.config.modwheel = normalized,
            2 => self.globals.breath = normalized,
            7 => self.master_volume = normalized,
            64 => {
                let sustain = value >= 64;
                self.manager.sustain = sustain;
                self.arpeggiator.set_sustain(sustain, &self.config.arpeggiator);
                if !sustain {
                    for (index, voice) in self.voices.iter_mut().enumerate() {
                        if self.manager.deferred[index] { voice.note_off(); }
                    }
                    self.manager.deferred.fill(false);
                    if !self.voices.iter().any(|voice| voice.active && !voice.is_releasing()) { self.global_env.release(); }
                }
            }
            67 => self.globals.soft_pedal = normalized,
            120 => self.all_sound_off(),
            123 => self.all_notes_off(),
            _ => {}
        }
    }

    fn pitch_bend(&mut self, value: i16) { self.config.pitchbend = value.clamp(-8192, 8191) as f32 / 8192.0; }

    fn process(&mut self, output: &mut [f32], channels: usize) {
        output.fill(0.0);
        if channels == 0 { return; }
        for frame in output.chunks_exact_mut(channels) {
            if self.config.arpeggiator.on {
                let ratio = crate::preset::sync_ratio(self.config.arpeggiator.sync_numerator,
                    self.config.arpeggiator.sync_denominator);
                let (on, off, count) = self.arpeggiator.tick(&self.config.arpeggiator, self.sample_rate,
                    self.config.bpm, ratio, self.previous_global_mod.get(970), self.previous_global_mod.get(973));
                if let Some(note) = on { self.start_note(note.note, note.velocity, note.modulation, false); }
                for &note in &off[..count] { self.stop_note(note, false); }
            }
            self.master_smooth = 0.995 * self.master_smooth + 0.005 * self.master_volume;
            self.globals.pitchwheel = 0.998 * self.globals.pitchwheel + 0.002 * self.config.pitchbend;
            self.globals.modwheel = 0.998 * self.globals.modwheel + 0.002 * self.config.modwheel;
            self.xy_smooth[0] = 0.998 * self.xy_smooth[0] + 0.002 * self.config.xy_x;
            self.xy_smooth[1] = 0.998 * self.xy_smooth[1] + 0.002 * self.config.xy_y;
            self.globals.x = (self.xy_smooth[0] + self.previous_global_mod.get(980)).clamp(0.0, 1.0);
            self.globals.y = (self.xy_smooth[1] + self.previous_global_mod.get(982)).clamp(0.0, 1.0);
            self.globals.sustain_pedal = if self.manager.sustain { 1.0 } else { 0.0 };
            // Matrice entière avant toute avancée des sources DSP (un sample de retard upstream).
            let previous = self.globals;
            let newest = self.manager.history[0];
            let sources: ModSources = *self.voices[newest].mod_sources();
            let modulation = self.mod_matrix.apply_global(&sources, &previous);
            let envelope = modulation.envelope(3);
            self.global_env.set_modulation([envelope.attack, envelope.decay, envelope.sustain, envelope.release]);
            self.globals.env = self.global_env.process();
            self.global_lfo.set_modulation(modulation.lfo(3));
            self.globals.lfo = self.global_lfo.process();
            let mut left = 0.0;
            let mut right = 0.0;
            for voice in &mut self.voices {
                voice.set_global_modulation(&modulation);
                let (l, r) = voice.process_with_modulation(&self.mod_matrix, &previous);
                left += l;
                right += r;
            }
            let filter_env = self.voices[newest].filter_env_value();
            let filter_mod = modulation.filter(2).into();
            for filter in &mut self.filter3 {
                filter.set_env_value(filter_env);
                filter.set_modulation(&filter_mod);
            }
            left = self.filter3[0].process(left);
            right = self.filter3[1].process(right);
            self.effects.set_modulation(&modulation);
            (left, right) = self.effects.process(left, right);
            let gain = modulate_gain(self.master_smooth, modulation.get(1000));
            frame[0] = left * gain;
            if channels > 1 { frame[1] = right * gain; }
            self.previous_global_mod = modulation;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for voice in &mut self.voices { voice.set_sample_rate(sample_rate); }
        for filter in &mut self.filter3 { filter.set_sample_rate(sample_rate); }
        self.global_env.set_sample_rate(sample_rate);
        self.global_lfo.set_sample_rate(sample_rate);
        self.effects.set_sample_rate(sample_rate);
    }
}

fn db_to_gain(db: f32) -> f32 {
    if db <= -100.0 { 0.0 } else { libm::powf(10.0, db / 20.0) }
}
