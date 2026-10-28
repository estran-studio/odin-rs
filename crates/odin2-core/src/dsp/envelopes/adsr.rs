//! ADSR Envelope Generator
//!
//! Based on the Odin 2 ADSREnvelope implementation.
//! Uses linear attack and exponential decay/release.

use super::{Envelope, EnvelopeState};
use libm::powf;

/// Minimum value before envelope is considered finished
const MIN_DECAY_RELEASE_VAL: f32 = 0.001;

/// Minimum attack time to avoid division by zero
const ATTACK_LOWER_LIMIT: f32 = 0.000001;

/// ADSR Envelope Generator
pub struct Adsr {
    /// Current envelope state
    state: EnvelopeState,

    /// Current value (internal, 0.0 to 1.0)
    current_value: f32,

    /// Last output value
    last_output: f32,

    /// Attack time in seconds
    attack: f32,

    /// Decay time in seconds
    decay: f32,

    /// Sustain level (0.0 to 1.0)
    sustain: f32,

    /// Release time in seconds
    release: f32,

    /// Sample rate
    sample_rate: f32,

    /// Cached decay factor
    decay_factor: f32,

    /// Last decay time (for caching)
    last_decay_time: f32,

    /// Cached release factor
    release_factor: f32,

    /// Last release time (for caching)
    last_release_time: f32,

    /// Value when release started
    release_start_value: f32,

    /// Whether envelope loops (for LFO-like behavior)
    looping: bool,

    modulation: [f32; 4],
}

impl Adsr {
    /// Create a new ADSR envelope
    pub fn new(sample_rate: f32) -> Self {
        Self {
            state: EnvelopeState::Idle,
            current_value: 0.0,
            last_output: 0.0,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
            sample_rate,
            decay_factor: 0.9998,
            last_decay_time: -1.0,
            release_factor: 0.9998,
            last_release_time: -1.0,
            release_start_value: 1.0,
            looping: false,
            modulation: [0.0; 4],
        }
    }

    /// Met à jour les contrôles sans réinitialiser l'enveloppe en cours.
    pub fn configure(&mut self, params: &crate::preset::EnvelopeParams) {
        self.set_attack(params.attack);
        self.set_decay(params.decay);
        self.set_sustain(params.sustain);
        self.set_release(params.release);
        self.set_looping(params.loop_env);
    }

    pub fn set_modulation(&mut self, modulation: [f32; 4]) {
        self.modulation = modulation;
    }

    /// En legato, une enveloppe tenue ne repart pas en attaque (ADSR.cpp:155).
    pub fn trigger_legato(&mut self) {
        self.current_value = self.last_output;
        if matches!(self.state, EnvelopeState::Idle | EnvelopeState::Release) {
            self.trigger();
        }
    }

    pub fn value(&self) -> f32 {
        self.last_output
    }

    fn modulated_time(time: f32, modulation: f32, release: bool) -> f32 {
        if modulation == 0.0 {
            time
        } else {
            (time * powf(2.0, (if release { 6.0 } else { 3.0 }) * modulation)
                + modulation * 0.3).max(ATTACK_LOWER_LIMIT)
        }
    }

    fn modulated_sustain(&self) -> f32 {
        (self.sustain + self.modulation[2]).clamp(0.0, 1.0)
    }
    /// Set attack time in seconds
    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack.max(ATTACK_LOWER_LIMIT);
    }

    /// Set decay time in seconds
    pub fn set_decay(&mut self, decay: f32) {
        self.decay = decay.max(ATTACK_LOWER_LIMIT);
    }

    /// Set sustain level (0.0 to 1.0)
    pub fn set_sustain(&mut self, sustain: f32) {
        self.sustain = sustain.clamp(0.0, 1.0);
    }

    /// Set release time in seconds
    pub fn set_release(&mut self, release: f32) {
        self.release = release.max(ATTACK_LOWER_LIMIT);
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        // Invalidate cached factors
        self.last_decay_time = -1.0;
        self.last_release_time = -1.0;
    }

    /// Set looping mode
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// Calculate decay/release factor
    ///
    /// Factor is computed so that: factor^(samples) = MIN_DECAY_RELEASE_VAL
    /// This gives an exponential curve that reaches MIN_DECAY_RELEASE_VAL
    /// in the specified time.
    #[inline]
    fn calc_exp_factor(&self, time: f32) -> f32 {
        let samples = self.sample_rate * time;
        powf(MIN_DECAY_RELEASE_VAL, 1.0 / samples)
    }

    /// Get decay factor (with caching)
    fn get_decay_factor(&mut self) -> f32 {
        let decay = Self::modulated_time(self.decay, self.modulation[1], false);
        if decay != self.last_decay_time {
            self.decay_factor = self.calc_exp_factor(decay);
            self.last_decay_time = decay;
        }
        self.decay_factor
    }

    /// Get release factor (with caching)
    fn get_release_factor(&mut self) -> f32 {
        let release = Self::modulated_time(self.release, self.modulation[3], true);
        if release != self.last_release_time {
            self.release_factor = self.calc_exp_factor(release);
            self.last_release_time = release;
        }
        self.release_factor
    }
}

impl Envelope for Adsr {
    fn process(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.last_output = 0.0;
                0.0
            }

            EnvelopeState::Attack => {
                // Linear attack
                let attack = Self::modulated_time(self.attack, self.modulation[0], false);
                let attack_inc = 1.0 / (self.sample_rate * attack);
                self.current_value += attack_inc;

                if self.current_value >= 1.0 {
                    self.current_value = 1.0;
                    self.state = EnvelopeState::Decay;
                }

                self.last_output = self.current_value;
                self.current_value
            }

            EnvelopeState::Decay => {
                // Exponential decay towards sustain
                let factor = self.get_decay_factor();
                self.current_value *= factor;
                let sustain = self.modulated_sustain();

                if self.current_value < MIN_DECAY_RELEASE_VAL {
                    if self.looping {
                        self.state = EnvelopeState::Attack;
                        self.current_value = sustain + (1.0 - sustain) * self.current_value;
                        self.last_output = sustain;
                        return sustain;
                    } else {
                        self.state = EnvelopeState::Sustain;
                        self.current_value = 0.0;
                    }
                }

                // Scale output: sustain + (1 - sustain) * current_value
                // This maps the exponential decay from 1->0 to 1->sustain
                self.last_output = sustain + (1.0 - sustain) * self.current_value;
                self.last_output
            }

            EnvelopeState::Sustain => {
                let sustain = self.modulated_sustain();
                if self.looping {
                    self.state = EnvelopeState::Attack;
                    self.current_value = sustain + (1.0 - sustain) * self.current_value;
                }
                self.last_output = sustain;
                sustain
            }

            EnvelopeState::Release => {
                // Exponential release towards zero
                let factor = self.get_release_factor();
                self.current_value *= factor;

                if self.current_value < MIN_DECAY_RELEASE_VAL {
                    self.current_value = 0.0;
                    self.state = EnvelopeState::Idle;
                    self.last_output = 0.0;
                    return 0.0;
                }

                // Scale output by the value when release started
                self.last_output = self.current_value * self.release_start_value;
                self.last_output
            }
        }
    }

    fn trigger(&mut self) {
        // Start from current value (for legato)
        self.current_value = self.last_output;
        self.state = EnvelopeState::Attack;
    }

    fn release(&mut self) {
        if self.state == EnvelopeState::Release {
            return;
        }

        // Store the current output value as the release starting point
        let sustain = self.modulated_sustain();
        match self.state {
            EnvelopeState::Decay => {
                self.release_start_value =
                    self.current_value * (1.0 - sustain) + sustain;
            }
            EnvelopeState::Sustain => {
                self.release_start_value = sustain;
            }
            _ => {
                self.release_start_value = self.last_output;
            }
        }

        self.current_value = 1.0;
        self.state = EnvelopeState::Release;
    }

    fn state(&self) -> EnvelopeState {
        self.state
    }

    fn is_active(&self) -> bool {
        self.state != EnvelopeState::Idle
    }

    fn reset(&mut self) {
        self.state = EnvelopeState::Idle;
        self.current_value = 0.0;
        self.last_output = 0.0;
        self.release_start_value = 1.0;
        self.last_decay_time = -1.0;
        self.last_release_time = -1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legato_keeps_sustain_but_restarts_release_from_current_output() {
        let mut env = Adsr::new(1000.0);
        env.set_attack(0.001);
        env.set_decay(0.001);
        env.set_sustain(0.4);
        env.set_release(1.0);
        env.trigger();
        for _ in 0..10 { env.process(); }
        assert_eq!(env.state(), EnvelopeState::Sustain);
        env.trigger_legato();
        assert_eq!(env.state(), EnvelopeState::Sustain);
        assert_eq!(env.process(), 0.4);
        env.release();
        let value = env.process();
        env.trigger_legato();
        assert_eq!(env.state(), EnvelopeState::Attack);
        assert_eq!(env.value(), value);
    }

    #[test]
    fn attack_modulation_changes_time_not_output_amplitude() {
        let mut env = Adsr::new(1000.0);
        env.set_attack(0.1);
        env.set_modulation([1.0, 0.0, 0.0, 0.0]);
        env.trigger();
        // ADSR.cpp:45: 0.1 * 2^3 + 0.3 = 1.1 seconde.
        let first = env.process();
        assert!((first - 1.0 / 1100.0).abs() < 1e-7);
        for _ in 0..1200 { env.process(); }
        assert!(env.value() <= 1.0);
        assert_ne!(env.state(), EnvelopeState::Attack);
    }

    #[test]
    fn test_adsr_trigger_and_release() {
        let mut env = Adsr::new(44100.0);
        env.set_attack(0.01);
        env.set_decay(0.1);
        env.set_sustain(0.5);
        env.set_release(0.1);

        // Initially idle
        assert_eq!(env.state(), EnvelopeState::Idle);
        assert!(!env.is_active());

        // Trigger
        env.trigger();
        assert_eq!(env.state(), EnvelopeState::Attack);
        assert!(env.is_active());

        // Process through attack
        let mut max_val = 0.0;
        for _ in 0..1000 {
            let val = env.process();
            if val > max_val {
                max_val = val;
            }
        }
        assert!(max_val > 0.9, "Attack should reach near 1.0");

        // Process through decay
        for _ in 0..10000 {
            env.process();
        }

        // Should be at sustain
        assert_eq!(env.state(), EnvelopeState::Sustain);
        let val = env.process();
        assert!((val - 0.5).abs() < 0.01, "Should be at sustain level");

        // Release
        env.release();
        assert_eq!(env.state(), EnvelopeState::Release);

        // Process through release
        for _ in 0..10000 {
            env.process();
        }

        // Should be idle
        assert_eq!(env.state(), EnvelopeState::Idle);
        assert!(!env.is_active());
    }

    #[test]
    fn test_adsr_range() {
        let mut env = Adsr::new(44100.0);
        env.set_attack(0.001);
        env.set_decay(0.05);
        env.set_sustain(0.7);
        env.set_release(0.05);

        env.trigger();

        for _ in 0..50000 {
            let val = env.process();
            assert!((0.0..=1.0).contains(&val), "Envelope out of range: {}", val);
        }
    }
}
