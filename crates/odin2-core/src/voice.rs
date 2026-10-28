//! Voix Odin: Process.cpp:147-360, Voice.h:184-230, Amplifier.cpp:24-69.

extern crate alloc;
use alloc::rc::Rc;
use crate::dsp::effects::OversamplingDistortion;
use crate::dsp::envelopes::{Adsr, Envelope};
use crate::dsp::filters::PresetFilter;
use crate::dsp::filters::preset_filter::{decibels, modulated_gain};
use crate::dsp::oscillators::{DrawTableBank, Lfo, OscillatorModulation, PresetOscillator};
use crate::mod_matrix::{GlobalSources, ModMatrix, ModSources, ModulationValues};
use crate::preset::{DistortionParams, EnvelopeParams, FilterParams, FilterType, LfoParams, OscillatorParams, OscillatorType, PlayMode};

/// Contrôles copiables utilisables par un hôte no_std. Gains en dB, glide normalisé.
#[derive(Debug, Clone, Copy)]
pub struct VoiceConfig {
    pub oscillators: [OscillatorParams; 3],
    pub filters: [FilterParams; 2],
    pub envelopes: [EnvelopeParams; 3],
    pub lfos: [LfoParams; 3],
    pub distortion: DistortionParams,
    pub amp_gain: f32,
    pub pan: f32,
    pub velocity_amount: f32,
    pub glide: f32,
    pub pitch_bend_range: f32,
    pub play_mode: PlayMode,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        let mut oscillators = [OscillatorParams::default(); 3];
        oscillators[0].volume = 0.0;
        oscillators[1].volume = -60.0;
        oscillators[2].volume = -60.0;
        let mut filters = [FilterParams::default(); 2];
        filters[0].filter_type = FilterType::None;
        filters[0].frequency = 20000.0;
        filters[0].osc1_input = true;
        filters[0].osc2_input = true;
        filters[0].osc3_input = true;
        filters[0].to_amp = true;
        filters[1].filter_type = FilterType::None;
        filters[1].frequency = 20000.0;
        let envelopes = [EnvelopeParams { attack: 0.01, decay: 0.1, sustain: 0.7,
            release: 0.3, loop_env: false }; 3];
        Self { oscillators, filters, envelopes, lfos: [LfoParams::default(); 3],
            distortion: DistortionParams::default(), amp_gain: 0.0, pan: 0.0,
            velocity_amount: 1.0, glide: 0.0, pitch_bend_range: 2.0, play_mode: PlayMode::Poly }
    }
}

pub struct Voice {
    pub oscillators: [PresetOscillator; 3],
    pub filter1: PresetFilter,
    pub filter2: PresetFilter,
    pub amp_env: Adsr,
    pub filter_env: Adsr,
    pub mod_env: Adsr,
    pub lfos: [Lfo; 3],
    pub note: u8,
    pub velocity: f32,
    pub active: bool,
    config: VoiceConfig,
    distortion: [OversamplingDistortion; 2],
    sources: ModSources,
    /// Fréquence de base qui glisse vers la cible: double comme `Oscillator::m_osc_freq_base`.
    frequency: f64,
    target_frequency: f64,
    sample_rate: f32,
    /// Contrôle de glide ajusté au samplerate (PluginProcessorValueChange.cpp:1100).
    glide_control: f32,
    unison_pan: f32,
    unison_detune: f32,
    unison_gain: f32,
    global_glide: f32,
    osc_gain_smooth: [f32; 3],
    osc_gain_target: [f32; 3],
    amp_gain_target: f32,
    amp_gain_smooth: f32,
    pan_smooth: f32,
    releasing: bool,
    random_state: u32,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        let mut voice = Self {
            oscillators: core::array::from_fn(|_| PresetOscillator::new(sample_rate)),
            filter1: PresetFilter::new(sample_rate), filter2: PresetFilter::new(sample_rate),
            amp_env: Adsr::new(sample_rate), filter_env: Adsr::new(sample_rate),
            mod_env: Adsr::new(sample_rate), lfos: core::array::from_fn(|_| Lfo::new(sample_rate)),
            note: 0, velocity: 0.0, active: false, config: VoiceConfig::default(),
            distortion: [OversamplingDistortion::new(), OversamplingDistortion::new()],
            sources: ModSources::default(), frequency: 440.0, target_frequency: 440.0,
            sample_rate, glide_control: 0.0,
            unison_pan: 0.0, unison_detune: 0.0, unison_gain: 1.0, global_glide: 0.0,
            osc_gain_smooth: [1.0; 3], amp_gain_smooth: 1.0, pan_smooth: 0.0,
            osc_gain_target: [1.0; 3], amp_gain_target: 1.0,
            releasing: false, random_state: 0x6d2b79f5,
        };
        voice.filter1.set_smoothing_state(20000.0, 1.0);
        voice.configure(&VoiceConfig::default());
        voice
    }

    /// Ne réinitialise ni phase, ni filtre, ni enveloppe; utilisable sur une voix tenue.
    pub fn configure(&mut self, config: &VoiceConfig) {
        for i in 0..3 {
            self.oscillators[i].set_oscillator_index(i);
            self.oscillators[i].configure(&config.oscillators[i]);
            self.lfos[i].configure(&config.lfos[i]);
            self.osc_gain_target[i] = decibels(config.oscillators[i].volume);
        }
        self.filter1.configure(&config.filters[0]);
        self.filter2.configure(&config.filters[1]);
        self.amp_env.configure(&config.envelopes[0]);
        self.filter_env.configure(&config.envelopes[1]);
        self.mod_env.configure(&config.envelopes[2]);
        for distortion in &mut self.distortion { distortion.configure(&config.distortion); }
        self.amp_gain_target = libm::powf(10.0, config.amp_gain / 20.0);
        self.config = *config;
        self.update_glide_control();
    }
    /// `glide_samplerate_adjusted = pow(glide, 44800. / samplerate)` (PluginProcessorValueChange.cpp:1100).
    fn update_glide_control(&mut self) {
        self.glide_control = libm::pow(self.config.glide as f64, 44800.0 / self.sample_rate as f64) as f32;
    }

    /// À appeler hors rendu après construction de la banque partagée par le moteur.
    pub fn set_draw_tables(&mut self, tables: Rc<DrawTableBank>) {
        for osc in &mut self.oscillators { osc.set_draw_tables(Rc::clone(&tables)); }
        for lfo in &mut self.lfos { lfo.set_draw_tables(Rc::clone(&tables)); }
    }

    pub fn set_tempo(&mut self, bpm: f32) {
        for lfo in &mut self.lfos { lfo.set_tempo(bpm); }
    }

    pub fn set_unison(&mut self, pan: f32, detune: f32, gain: f32) {
        self.unison_pan = pan;
        self.unison_detune = detune;
        self.unison_gain = gain;
        self.sources.unison_position = pan;
    }

    /// Détune en demi-tons; pan déjà multiplié par la largeur d'unisson du patch.
    #[allow(clippy::too_many_arguments)]
    pub fn start(&mut self, note: u8, velocity: u8, last_note: Option<u8>,
        unison_pan: f32, unison_detune: f32, unison_gain: f32, legato: bool) {
        let was_active = self.active;
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.active = true;
        self.releasing = false;
        // Voice::start → setOscBaseFreq(float, float) (Voice.h:359–364).
        self.target_frequency = crate::dsp::midi_to_freq(note) as f64;
        self.frequency = last_note.map(|last| crate::dsp::midi_to_freq(last) as f64).unwrap_or(self.target_frequency);
        self.set_unison(unison_pan, unison_detune, unison_gain);
        self.sources.note = note as f32 / 127.0;
        self.sources.velocity = self.velocity;
        self.sources.aftertouch = 0.0;
        self.random_state ^= self.random_state << 13;
        self.random_state ^= self.random_state >> 17;
        self.random_state ^= self.random_state << 5;
        self.sources.random = self.random_state as f32 / u32::MAX as f32 * 2.0 - 1.0;
        self.filter1.set_midi(note, velocity);
        self.filter2.set_midi(note, velocity);
        let poly = self.config.play_mode == PlayMode::Poly && !legato;
        if poly {
            self.filter1.reset();
            self.filter2.reset();
        }
        for i in 0..3 {
            self.oscillators[i].voice_start(!was_active &&
                (unison_gain != 1.0 || self.config.oscillators[i].osc_type == OscillatorType::Multi));
            self.lfos[i].retrigger();
        }
        for env in [&mut self.amp_env, &mut self.filter_env, &mut self.mod_env] {
            if legato || self.config.play_mode == PlayMode::Legato {
                if was_active { env.trigger_legato(); } else { env.reset(); env.trigger(); }
            } else {
                if poly { env.reset(); }
                env.trigger();
            }
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.start(note, velocity, None, 0.0, 0.0, 1.0, false);
    }

    pub fn note_off(&mut self) {
        self.amp_env.release(); self.filter_env.release(); self.mod_env.release();
        self.releasing = true;
    }
    pub fn is_releasing(&self) -> bool { self.releasing }
    pub fn filter_env_value(&self) -> f32 { self.sources.env[1] }
    pub fn mod_sources(&self) -> &ModSources { &self.sources }
    pub fn set_aftertouch(&mut self, value: f32) { self.sources.aftertouch = value; }
    pub fn set_arp_modulation(&mut self, values: [f32; 2]) { self.sources.arp_mod = values; }
    pub fn set_unison_position(&mut self, position: f32) { self.sources.unison_position = position; }
    pub fn set_seed(&mut self, seed: u32) {
        self.random_state = seed.max(1);
        for (i, osc) in self.oscillators.iter_mut().enumerate() {
            osc.set_seed(seed.wrapping_add((i as u32 + 1).wrapping_mul(0x9e3779b9)));
        }
    }
    pub fn set_global_modulation(&mut self, values: &ModulationValues) {
        self.global_glide = values.get(999);
    }

    pub fn process(&mut self) -> (f32, f32) {
        self.process_values(&ModulationValues::default(), &GlobalSources::default())
    }

    /// La matrice lit les sources de la trame précédente, comme Process.cpp:125.
    pub fn process_with_modulation(&mut self, matrix: &ModMatrix, globals: &GlobalSources) -> (f32, f32) {
        let values = matrix.apply_voice(&self.sources, globals);
        self.process_values(&values, globals)
    }

    fn process_values(&mut self, values: &ModulationValues, globals: &GlobalSources) -> (f32, f32) {
        for i in 0..3 {
            self.osc_gain_smooth[i] = self.osc_gain_smooth[i] * 0.995
                + self.osc_gain_target[i] * (1.0 - 0.995);
        }
        self.sources.osc = [0.0; 3];
        self.sources.filter = [0.0; 2];
        if !self.active {
            self.filter1.advance_controls();
            self.filter2.advance_controls();
            return (0.0, 0.0);
        }
        for (i, env) in [&mut self.amp_env, &mut self.filter_env, &mut self.mod_env].into_iter().enumerate() {
            let m = values.envelope(i);
            env.set_modulation([m.attack, m.decay, m.sustain, m.release]);
            self.sources.env[i] = env.process();
            self.lfos[i].set_modulation(values.lfo(i));
            self.sources.lfo[i] = self.lfos[i].process();
        }
        if !self.amp_env.is_active() { self.active = false; return (0.0, 0.0); }
        // Oscillator::update / calculateGlide (Oscillator.h:35–73), en double.
        let glide = (self.glide_control + self.global_glide).min(1.0);
        let coefficient = if glide < 0.01 { 0.0 } else { 0.9985f32 as f64 + glide as f64 * 0.0014 };
        self.frequency = self.target_frequency * (1.0f32 as f64 - coefficient) + coefficient * self.frequency;
        // unison_detune_factor = pow(2.f, position * amount / 12.f) (Voice.h:574)
        let frequency = self.frequency * libm::powf(2.0, self.unison_detune / 12.0) as f64;
        // m_pitch_bend_smooth_and_applied, en demi-tons, entre dans la somme exponentielle de l'oscillateur.
        let pitch_bend = globals.pitchwheel * self.config.pitch_bend_range;
        let mut master_wrap = false;
        let mut master_position = 0.0;
        for i in 0..3 {
            let m = values.osc(i);
            let osc = &mut self.oscillators[i];
            osc.set_frequency_f64(frequency);
            osc.set_sync_input(i != 0 && master_wrap);
            osc.set_sync_position(master_position);
            osc.set_position_source(if self.config.oscillators[i].mod_source == 10 {
                self.sources.lfo[0]
            } else { self.sources.env[2] });
            osc.set_modulation(OscillatorModulation {
                pitch_semitones: pitch_bend, exponential_fm: m.pitch_exponential, linear_fm: m.pitch_linear,
                phase: 0.0, position: m.position, pulse_width: m.pulse_width,
                vector_x: m.x, vector_y: m.y, fm_amount: m.fm_amount,
                fm_ratio: m.modulator_ratio, carrier_ratio: m.carrier_ratio,
                multi_detune: m.detune, spread: m.spread, arp_speed: m.arp_speed,
                noise_hp: m.hp_freq, noise_lp: m.lp_freq, drift: 0.0,
            });
            let signal = osc.process();
            if i == 0 {
                master_wrap = osc.did_wrap();
                master_position = osc.reset_position();
            }
            self.sources.osc[i] = signal * modulated_gain(self.osc_gain_smooth[i], m.vol);
        }
        let mut input = [0.0; 2];
        for i in 0..2 {
            let p = &self.config.filters[i];
            if p.osc1_input { input[i] += self.sources.osc[0]; }
            if p.osc2_input { input[i] += self.sources.osc[1]; }
            if p.osc3_input { input[i] += self.sources.osc[2]; }
        }
        self.filter1.set_env_value(self.sources.env[1]);
        self.filter2.set_env_value(self.sources.env[1]);
        self.filter1.set_modulation(&values.filter(0).into());
        self.filter2.set_modulation(&values.filter(1).into());
        self.sources.filter[0] = self.filter1.process(input[0]);
        if self.config.filters[1].fil1_input { input[1] += self.sources.filter[0]; }
        self.sources.filter[1] = self.filter2.process(input[1]);
        let mut signal = 0.0;
        if self.config.filters[0].to_amp { signal += self.sources.filter[0]; }
        if self.config.filters[1].to_amp { signal += self.sources.filter[1]; }
        self.amp_gain_smooth = self.amp_gain_smooth * 0.995
            + self.amp_gain_target * (1.0 - 0.995);
        self.pan_smooth = self.pan_smooth * 0.998 + self.config.pan * (1.0 - 0.998);
        let velocity_amount = (self.config.velocity_amount + values.get(902)).clamp(0.0, 1.0);
        signal *= modulated_gain(self.amp_gain_smooth, values.get(900)) * self.unison_gain
            * (1.0 - velocity_amount + velocity_amount * self.velocity);
        let pan = (self.pan_smooth + values.get(901) + self.unison_pan).clamp(-1.0, 1.0);
        let mut left = signal * if pan >= 0.0 { 1.0 - pan } else { 1.0 };
        let mut right = signal * if pan < 0.0 { 1.0 + pan } else { 1.0 };
        if self.config.distortion.on {
            for distortion in &mut self.distortion { distortion.set_modulation(values.get(950), values.get(951)); }
            left = self.distortion[0].process(left);
            right = self.distortion[1].process(right);
        }
        (left * self.sources.env[0], right * self.sources.env[0])
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_glide_control();
        for osc in &mut self.oscillators { osc.set_sample_rate(sample_rate); }
        for lfo in &mut self.lfos { lfo.set_sample_rate(sample_rate); }
        self.filter1.set_sample_rate(sample_rate); self.filter2.set_sample_rate(sample_rate);
        self.amp_env.set_sample_rate(sample_rate); self.filter_env.set_sample_rate(sample_rate);
        self.mod_env.set_sample_rate(sample_rate);
        for distortion in &mut self.distortion { distortion.set_sample_rate(sample_rate); }
    }

    pub fn reset(&mut self) {
        self.active = false; self.note = 0; self.velocity = 0.0; self.releasing = false;
        self.sources = ModSources::default();
        for osc in &mut self.oscillators { osc.reset(); }
        for lfo in &mut self.lfos { lfo.reset(); }
        self.filter1.reset(); self.filter2.reset();
        self.amp_env.reset(); self.filter_env.reset(); self.mod_env.reset();
        for distortion in &mut self.distortion { distortion.reset(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_none_paths_sum_without_oscillator_normalization() {
        let mut config = VoiceConfig::default();
        config.oscillators[0].reset = true;
        config.oscillators[0].drift = 0.0;
        let mut single = Voice::new(44100.0);
        let mut parallel = Voice::new(44100.0);
        single.configure(&config);
        config.filters[1].osc1_input = true;
        config.filters[1].osc2_input = true;
        config.filters[1].osc3_input = true;
        config.filters[1].to_amp = true;
        parallel.configure(&config);
        single.note_on(60, 100);
        parallel.note_on(60, 100);
        for _ in 0..512 {
            let (a, _) = single.process();
            let (b, _) = parallel.process();
            assert_eq!(b, a * 2.0);
        }
    }

    #[test]
    fn changing_a_held_parameter_does_not_retrigger_the_envelope() {
        let mut voice = Voice::new(44100.0);
        let mut config = VoiceConfig { play_mode: PlayMode::Legato, ..Default::default() };
        voice.configure(&config);
        voice.note_on(60, 100);
        for _ in 0..1000 { voice.process(); }
        let state = voice.amp_env.state();
        let value = voice.amp_env.value();
        config.pan = 0.75;
        config.filters[0].frequency = 2000.0;
        voice.configure(&config);
        assert_eq!(voice.amp_env.state(), state);
        assert_eq!(voice.amp_env.value(), value);
        voice.start(67, 90, Some(60), 0.0, 0.0, 1.0, true);
        assert_eq!(voice.amp_env.state(), state);
        assert_eq!(voice.amp_env.value(), value);
    }
}
