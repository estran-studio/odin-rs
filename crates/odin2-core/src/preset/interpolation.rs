//! Preset Interpolation
//!
//! Provides smooth morphing between Odin 2 presets for procedural sound design.

extern crate alloc;
use alloc::format;
use alloc::string::ToString;

use super::parser::OdinPreset;
use super::types::*;

// ============================================================================
// Helper Functions
// ============================================================================

/// Linear interpolation for f32 values
#[inline]
fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

/// Linear interpolation for i32 values (with rounding)
#[inline]
fn lerp_i32(a: i32, b: i32, t: f32) -> i32 {
    let result = a as f32 * (1.0 - t) + b as f32 * t;
    result.round() as i32
}

/// Switch between two values at t=0.5
#[inline]
fn switch_at_half<T: Clone>(a: &T, b: &T, t: f32) -> T {
    if t < 0.5 {
        a.clone()
    } else {
        b.clone()
    }
}

/// Smoothstep function for ease-in-out interpolation
#[inline]
fn smooth_step(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ============================================================================
// OdinPreset Interpolation Implementation
// ============================================================================

impl OdinPreset {
    /// Linear interpolation between two presets
    ///
    /// # Arguments
    /// * `other` - The target preset to interpolate towards
    /// * `t` - Interpolation factor (0.0 = self, 1.0 = other)
    ///
    /// # Examples
    /// ```
    /// use odin2_core::preset::OdinPreset;
    ///
    /// let p1 = OdinPreset::default();
    /// let p2 = OdinPreset::default();
    /// // ... set some values ...
    /// let mixed = p1.interpolate(&p2, 0.3); // 30% toward p2
    /// ```
    pub fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.interpolate_linear(other, t)
    }

    /// Smooth interpolation with ease-in-out curve
    ///
    /// Uses smoothstep function for more natural-sounding transitions
    pub fn interpolate_smooth(&self, other: &Self, t: f32) -> Self {
        let smooth_t = smooth_step(t);
        self.interpolate_linear(other, smooth_t)
    }

    /// Mix multiple presets with weights
    pub fn mix_presets(presets: &[&OdinPreset], weights: &[f32]) -> Self {
        if presets.is_empty() {
            return Self::default();
        }

        // Check for dominant weight to preserve name
        let mut dominant_idx = None;
        for (i, &w) in weights.iter().enumerate() {
            if w > 0.99 {
                dominant_idx = Some(i);
                break;
            }
        }

        // If one preset has basically all weight, return it directly
        if let Some(idx) = dominant_idx {
            return presets[idx].clone();
        }

        // Recursively mix components using linear interpolation
        let mut running_mix = presets[0].clone();
        
        if presets.len() > 1 {
            let mut current_weight_sum = weights[0];
            
            for i in 1..presets.len() {
                let next_weight = weights[i];
                if next_weight < 0.00001 { continue; }

                let total = current_weight_sum + next_weight;
                if total > 0.0 {
                    let ratio = next_weight / total;
                    running_mix = running_mix.interpolate(presets[i], ratio);
                }
                current_weight_sum = total;
            }
        }
        
        // precise name
        running_mix.name = "Morph 2D".to_string();
        
        running_mix
    }

    /// Core linear interpolation implementation
    fn interpolate_linear(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);

        // Generate descriptive name
        let name = if t == 0.0 {
            self.name.clone()
        } else if t == 1.0 {
            other.name.clone()
        } else {
            format!("{} → {} ({:.0}%)", self.name, other.name, t * 100.0)
        };

        OdinPreset {
            name,
            version_minor: self.version_minor,
            version_patch: self.version_patch,
            patch_migration_version: switch_at_half(&self.patch_migration_version, &other.patch_migration_version, t),
            bpm: lerp_f32(self.bpm, other.bpm, t),
            fx_positions: switch_at_half(&self.fx_positions, &other.fx_positions, t),
            modwheel: lerp_f32(self.modwheel, other.modwheel, t),
            pitchbend: lerp_f32(self.pitchbend, other.pitchbend, t),
            ui: switch_at_half(&self.ui, &other.ui, t),

            // === Oscillators ===
            osc1: interpolate_oscillator(&self.osc1, &other.osc1, t),
            osc2: interpolate_oscillator(&self.osc2, &other.osc2, t),
            osc3: interpolate_oscillator(&self.osc3, &other.osc3, t),

            // === Filters ===
            filter1: interpolate_filter(&self.filter1, &other.filter1, t),
            filter2: interpolate_filter(&self.filter2, &other.filter2, t),
            filter3: interpolate_filter(&self.filter3, &other.filter3, t),

            // === Envelopes ===
            env1: interpolate_envelope(&self.env1, &other.env1, t),
            env2: interpolate_envelope(&self.env2, &other.env2, t),
            env3: interpolate_envelope(&self.env3, &other.env3, t),
            env4: interpolate_envelope(&self.env4, &other.env4, t),

            // === LFOs ===
            lfo1: interpolate_lfo(&self.lfo1, &other.lfo1, t),
            lfo2: interpolate_lfo(&self.lfo2, &other.lfo2, t),
            lfo3: interpolate_lfo(&self.lfo3, &other.lfo3, t),
            lfo4: interpolate_lfo(&self.lfo4, &other.lfo4, t),

            // === Effects ===
            delay: interpolate_delay(&self.delay, &other.delay, t),
            reverb: interpolate_reverb(&self.reverb, &other.reverb, t),
            chorus: interpolate_chorus(&self.chorus, &other.chorus, t),
            phaser: interpolate_phaser(&self.phaser, &other.phaser, t),
            flanger: interpolate_flanger(&self.flanger, &other.flanger, t),
            distortion: interpolate_distortion(&self.distortion, &other.distortion, t),

            // === Modulation Matrix ===
            mod_matrix: interpolate_mod_matrix(&self.mod_matrix, &other.mod_matrix, t),

            // === Arpeggiator ===
            arpeggiator: interpolate_arpeggiator(&self.arpeggiator, &other.arpeggiator, t),

            // === Global Parameters ===
            master: lerp_f32(self.master, other.master, t),
            glide: lerp_f32(self.glide, other.glide, t),
            unison_detune: lerp_f32(self.unison_detune, other.unison_detune, t),
            unison_width: lerp_f32(self.unison_width, other.unison_width, t),
            unison_voices: lerp_i32(self.unison_voices, other.unison_voices, t),
            play_mode: switch_at_half(&self.play_mode, &other.play_mode, t),
            pitchbend_amount: lerp_i32(self.pitchbend_amount, other.pitchbend_amount, t),

            // === Amp Section ===
            amp_velocity: lerp_f32(self.amp_velocity, other.amp_velocity, t),
            amp_pan: lerp_f32(self.amp_pan, other.amp_pan, t),
            amp_gain: lerp_f32(self.amp_gain, other.amp_gain, t),

            // === XY Pad ===
            xy_x: lerp_f32(self.xy_x, other.xy_x, t),
            xy_y: lerp_f32(self.xy_y, other.xy_y, t),
        }
    }
}

// ============================================================================
// Component Interpolation Functions
// ============================================================================

fn interpolate_oscillator(a: &OscillatorParams, b: &OscillatorParams, t: f32) -> OscillatorParams {
    OscillatorParams {
        // Discrete parameters (switch at 0.5)
        osc_type: switch_at_half(&a.osc_type, &b.osc_type, t),
        analog_wave: switch_at_half(&a.analog_wave, &b.analog_wave, t),
        reset: switch_at_half(&a.reset, &b.reset, t),
        sync: switch_at_half(&a.sync, &b.sync, t),
        arp_on: switch_at_half(&a.arp_on, &b.arp_on, t),
        step_3_on: switch_at_half(&a.step_3_on, &b.step_3_on, t),
        chipnoise: switch_at_half(&a.chipnoise, &b.chipnoise, t),
        exp_fm: switch_at_half(&a.exp_fm, &b.exp_fm, t),
        wavedraw: core::array::from_fn(|i| lerp_f32(a.wavedraw[i], b.wavedraw[i], t)),
        chipdraw: core::array::from_fn(|i| lerp_f32(a.chipdraw[i], b.chipdraw[i], t)),
        specdraw: core::array::from_fn(|i| lerp_f32(a.specdraw[i], b.specdraw[i], t)),

        // Integer parameters (lerp + round)
        octave: lerp_i32(a.octave, b.octave, t),
        semitones: lerp_i32(a.semitones, b.semitones, t),
        wavetable: switch_at_half(&a.wavetable, &b.wavetable, t),
        chipwave: switch_at_half(&a.chipwave, &b.chipwave, t),
        vec_a: switch_at_half(&a.vec_a, &b.vec_a, t),
        vec_b: switch_at_half(&a.vec_b, &b.vec_b, t),
        vec_c: switch_at_half(&a.vec_c, &b.vec_c, t),
        vec_d: switch_at_half(&a.vec_d, &b.vec_d, t),
        modulator_wave: switch_at_half(&a.modulator_wave, &b.modulator_wave, t),
        carrier_wave: switch_at_half(&a.carrier_wave, &b.carrier_wave, t),
        mod_source: switch_at_half(&a.mod_source, &b.mod_source, t),
        carrier_ratio: switch_at_half(&a.carrier_ratio, &b.carrier_ratio, t),
        modulator_ratio: switch_at_half(&a.modulator_ratio, &b.modulator_ratio, t),
        step_1: lerp_i32(a.step_1, b.step_1, t),
        step_2: lerp_i32(a.step_2, b.step_2, t),
        step_3: lerp_i32(a.step_3, b.step_3, t),

        // Continuous parameters (linear interpolation)
        fine: lerp_f32(a.fine, b.fine, t),
        volume: lerp_f32(a.volume, b.volume, t),
        position: lerp_f32(a.position, b.position, t),
        detune: lerp_f32(a.detune, b.detune, t),
        pos_mod: lerp_f32(a.pos_mod, b.pos_mod, t),
        multi_position: lerp_f32(a.multi_position, b.multi_position, t),
        spread: lerp_f32(a.spread, b.spread, t),
        pulsewidth: lerp_f32(a.pulsewidth, b.pulsewidth, t),
        drift: lerp_f32(a.drift, b.drift, t),
        vec_x: lerp_f32(a.vec_x, b.vec_x, t),
        vec_y: lerp_f32(a.vec_y, b.vec_y, t),
        arp_speed: lerp_f32(a.arp_speed, b.arp_speed, t),
        fm: lerp_f32(a.fm, b.fm, t),
        lp: lerp_f32(a.lp, b.lp, t),
        hp: lerp_f32(a.hp, b.hp, t),
    }
}

fn interpolate_filter(a: &FilterParams, b: &FilterParams, t: f32) -> FilterParams {
    FilterParams {
        // Discrete parameters
        filter_type: switch_at_half(&a.filter_type, &b.filter_type, t),
        comb_polarity: switch_at_half(&a.comb_polarity, &b.comb_polarity, t),
        osc1_input: switch_at_half(&a.osc1_input, &b.osc1_input, t),
        osc2_input: switch_at_half(&a.osc2_input, &b.osc2_input, t),
        osc3_input: switch_at_half(&a.osc3_input, &b.osc3_input, t),
        fil1_input: switch_at_half(&a.fil1_input, &b.fil1_input, t),
        to_amp: switch_at_half(&a.to_amp, &b.to_amp, t),

        // Integer parameters
        vowel_left: lerp_i32(a.vowel_left, b.vowel_left, t),
        vowel_right: lerp_i32(a.vowel_right, b.vowel_right, t),

        // Continuous parameters with safety clamping
        frequency: lerp_f32(a.frequency, b.frequency, t).clamp(20.0, 20000.0),
        resonance: lerp_f32(a.resonance, b.resonance, t),
        saturation: lerp_f32(a.saturation, b.saturation, t),
        gain: lerp_f32(a.gain, b.gain, t),
        env_amount: lerp_f32(a.env_amount, b.env_amount, t),
        vel_amount: lerp_f32(a.vel_amount, b.vel_amount, t),
        kbd_follow: lerp_f32(a.kbd_follow, b.kbd_follow, t),
        sem_transition: lerp_f32(a.sem_transition, b.sem_transition, t),
        formant_transition: lerp_f32(a.formant_transition, b.formant_transition, t),
        ring_mod_amount: lerp_f32(a.ring_mod_amount, b.ring_mod_amount, t),
    }
}

fn interpolate_envelope(a: &EnvelopeParams, b: &EnvelopeParams, t: f32) -> EnvelopeParams {
    EnvelopeParams {
        // Discrete parameters
        loop_env: switch_at_half(&a.loop_env, &b.loop_env, t),

        // Continuous parameters with safety clamping
        attack: lerp_f32(a.attack, b.attack, t).max(0.001),
        decay: lerp_f32(a.decay, b.decay, t).max(0.001),
        sustain: lerp_f32(a.sustain, b.sustain, t),
        release: lerp_f32(a.release, b.release, t).max(0.01),
    }
}

fn interpolate_lfo(a: &LfoParams, b: &LfoParams, t: f32) -> LfoParams {
    LfoParams {
        // Discrete parameters
        waveform: switch_at_half(&a.waveform, &b.waveform, t),
        reset: switch_at_half(&a.reset, &b.reset, t),
        sync: switch_at_half(&a.sync, &b.sync, t),

        // Integer parameters
        sync_numerator: switch_at_half(&a.sync_numerator, &b.sync_numerator, t),
        sync_denominator: switch_at_half(&a.sync_denominator, &b.sync_denominator, t),

        // Continuous parameters
        frequency: lerp_f32(a.frequency, b.frequency, t),
    }
}

fn interpolate_delay(a: &DelayParams, b: &DelayParams, t: f32) -> DelayParams {
    DelayParams {
        // Discrete parameters
        on: switch_at_half(&a.on, &b.on, t),
        sync: switch_at_half(&a.sync, &b.sync, t),
        pingpong: switch_at_half(&a.pingpong, &b.pingpong, t),
        sync_numerator: switch_at_half(&a.sync_numerator, &b.sync_numerator, t),
        sync_denominator: switch_at_half(&a.sync_denominator, &b.sync_denominator, t),

        // Continuous parameters
        time: lerp_f32(a.time, b.time, t),
        feedback: lerp_f32(a.feedback, b.feedback, t),
        dry: lerp_f32(a.dry, b.dry, t),
        wet: lerp_f32(a.wet, b.wet, t),
        hp: lerp_f32(a.hp, b.hp, t),
        ducking: lerp_f32(a.ducking, b.ducking, t),
    }
}

fn interpolate_reverb(a: &ReverbParams, b: &ReverbParams, t: f32) -> ReverbParams {
    ReverbParams {
        // Discrete parameters
        on: switch_at_half(&a.on, &b.on, t),

        // Continuous parameters
        delay: lerp_f32(a.delay, b.delay, t),
        dry_wet: lerp_f32(a.dry_wet, b.dry_wet, t),
        mid_hall: lerp_f32(a.mid_hall, b.mid_hall, t),
        hf_damp: lerp_f32(a.hf_damp, b.hf_damp, t),
        eq_gain: lerp_f32(a.eq_gain, b.eq_gain, t),
        eq_freq: lerp_f32(a.eq_freq, b.eq_freq, t),
    }
}

fn interpolate_chorus(a: &ChorusParams, b: &ChorusParams, t: f32) -> ChorusParams {
    ChorusParams {
        // Discrete parameters
        on: switch_at_half(&a.on, &b.on, t),
        sync: switch_at_half(&a.sync, &b.sync, t),
        reset: switch_at_half(&a.reset, &b.reset, t),
        sync_numerator: switch_at_half(&a.sync_numerator, &b.sync_numerator, t),
        sync_denominator: switch_at_half(&a.sync_denominator, &b.sync_denominator, t),

        // Continuous parameters
        amount: lerp_f32(a.amount, b.amount, t),
        rate: lerp_f32(a.rate, b.rate, t),
        feedback: lerp_f32(a.feedback, b.feedback, t),
        dry_wet: lerp_f32(a.dry_wet, b.dry_wet, t),
    }
}

fn interpolate_phaser(a: &PhaserParams, b: &PhaserParams, t: f32) -> PhaserParams {
    PhaserParams {
        // Discrete parameters
        on: switch_at_half(&a.on, &b.on, t),
        sync: switch_at_half(&a.sync, &b.sync, t),
        reset: switch_at_half(&a.reset, &b.reset, t),
        sync_numerator: switch_at_half(&a.sync_numerator, &b.sync_numerator, t),
        sync_denominator: switch_at_half(&a.sync_denominator, &b.sync_denominator, t),

        // Continuous parameters
        frequency: lerp_f32(a.frequency, b.frequency, t),
        feedback: lerp_f32(a.feedback, b.feedback, t),
        rate: lerp_f32(a.rate, b.rate, t),
        mod_amount: lerp_f32(a.mod_amount, b.mod_amount, t),
        dry_wet: lerp_f32(a.dry_wet, b.dry_wet, t),
    }
}

fn interpolate_flanger(a: &FlangerParams, b: &FlangerParams, t: f32) -> FlangerParams {
    FlangerParams {
        // Discrete parameters
        on: switch_at_half(&a.on, &b.on, t),
        sync: switch_at_half(&a.sync, &b.sync, t),
        reset: switch_at_half(&a.reset, &b.reset, t),
        sync_numerator: switch_at_half(&a.sync_numerator, &b.sync_numerator, t),
        sync_denominator: switch_at_half(&a.sync_denominator, &b.sync_denominator, t),

        // Continuous parameters
        amount: lerp_f32(a.amount, b.amount, t),
        rate: lerp_f32(a.rate, b.rate, t),
        feedback: lerp_f32(a.feedback, b.feedback, t),
        dry_wet: lerp_f32(a.dry_wet, b.dry_wet, t),
    }
}

fn interpolate_distortion(a: &DistortionParams, b: &DistortionParams, t: f32) -> DistortionParams {
    DistortionParams {
        // Discrete parameters
        on: switch_at_half(&a.on, &b.on, t),
        algorithm: switch_at_half(&a.algorithm, &b.algorithm, t),

        // Continuous parameters
        boost: lerp_f32(a.boost, b.boost, t),
        dry_wet: lerp_f32(a.dry_wet, b.dry_wet, t),
    }
}

fn interpolate_mod_matrix(
    a: &[ModMatrixRow; 9],
    b: &[ModMatrixRow; 9],
    t: f32,
) -> [ModMatrixRow; 9] {
    core::array::from_fn(|i| ModMatrixRow {
        // Switch sources/dests at 0.5
        source: switch_at_half(&a[i].source, &b[i].source, t),
        dest1: switch_at_half(&a[i].dest1, &b[i].dest1, t),
        dest2: switch_at_half(&a[i].dest2, &b[i].dest2, t),
        scale: switch_at_half(&a[i].scale, &b[i].scale, t),

        // Interpolate amounts smoothly
        dest1_amount: lerp_f32(a[i].dest1_amount, b[i].dest1_amount, t),
        dest2_amount: lerp_f32(a[i].dest2_amount, b[i].dest2_amount, t),
        scale_amount: lerp_f32(a[i].scale_amount, b[i].scale_amount, t),
    })
}

fn interpolate_arpeggiator(
    a: &ArpeggiatorParams,
    b: &ArpeggiatorParams,
    t: f32,
) -> ArpeggiatorParams {
    ArpeggiatorParams {
        // Discrete parameters
        on: switch_at_half(&a.on, &b.on, t),
        direction: switch_at_half(&a.direction, &b.direction, t),
        one_shot: switch_at_half(&a.one_shot, &b.one_shot, t),

        // Integer parameters
        octaves: lerp_i32(a.octaves, b.octaves, t),
        steps: lerp_i32(a.steps, b.steps, t),
        gate: lerp_i32(a.gate, b.gate, t),
        sync_numerator: lerp_i32(a.sync_numerator, b.sync_numerator, t),
        sync_denominator: lerp_i32(a.sync_denominator, b.sync_denominator, t),

        // Continuous parameters
        mod_transpose: lerp_f32(a.mod_transpose, b.mod_transpose, t),

        // Interpolate each arp step
        step_params: core::array::from_fn(|i| ArpStep {
            on: switch_at_half(&a.step_params[i].on, &b.step_params[i].on, t),
            transpose: lerp_i32(a.step_params[i].transpose, b.step_params[i].transpose, t),
            mod1: lerp_f32(a.step_params[i].mod1, b.step_params[i].mod1, t),
            mod2: lerp_f32(a.step_params[i].mod2, b.step_params[i].mod2, t),
        }),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morph_preserves_discontinuous_ids_and_interpolates_drawings() {
        let mut a = OdinPreset::default();
        let mut b = OdinPreset::default();
        a.mod_matrix[0].source = 200;
        b.mod_matrix[0].source = 999;
        a.osc1.vec_a = 101;
        b.osc1.vec_a = 601;
        a.osc1.wavedraw[17] = -1.0;
        b.osc1.wavedraw[17] = 1.0;
        b.fx_positions = [4, 3, 2, 1, 0];
        let first = a.interpolate(&b, 0.25);
        assert_eq!(first.mod_matrix[0].source, 200);
        assert_eq!(first.osc1.vec_a, 101);
        assert_eq!(first.osc1.wavedraw[17], -0.5);
        assert_eq!(first.fx_positions, a.fx_positions);
        let second = a.interpolate(&b, 0.75);
        assert_eq!(second.mod_matrix[0].source, 999);
        assert_eq!(second.osc1.vec_a, 601);
        assert_eq!(second.fx_positions, b.fx_positions);
    }

    #[test]
    fn test_lerp_f32() {
        assert_eq!(lerp_f32(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp_f32(0.0, 10.0, 1.0), 10.0);
        assert_eq!(lerp_f32(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp_f32(10.0, 20.0, 0.25), 12.5);
    }

    #[test]
    fn test_lerp_i32() {
        assert_eq!(lerp_i32(0, 10, 0.0), 0);
        assert_eq!(lerp_i32(0, 10, 1.0), 10);
        assert_eq!(lerp_i32(0, 10, 0.5), 5);
        assert_eq!(lerp_i32(10, 20, 0.3), 13); // 13 (rounded from 13.0)
    }

    #[test]
    fn test_switch_at_half() {
        assert_eq!(switch_at_half(&10, &20, 0.0), 10);
        assert_eq!(switch_at_half(&10, &20, 0.4), 10);
        assert_eq!(switch_at_half(&10, &20, 0.5), 20);
        assert_eq!(switch_at_half(&10, &20, 1.0), 20);
    }

    #[test]
    fn test_smooth_step() {
        assert_eq!(smooth_step(0.0), 0.0);
        assert_eq!(smooth_step(1.0), 1.0);
        let mid = smooth_step(0.5);
        assert!(mid > 0.4 && mid < 0.6); // Should be around 0.5 but with smoothing
    }

    #[test]
    fn test_interpolate_extremes() {
        let preset_a = OdinPreset::default();
        let preset_b = OdinPreset::default();

        // t=0.0 should return preset_a
        let result = preset_a.interpolate(&preset_b, 0.0);
        assert_eq!(result.name, preset_a.name);

        // t=1.0 should return preset_b
        let result = preset_a.interpolate(&preset_b, 1.0);
        assert_eq!(result.name, preset_b.name);
    }

    #[test]
    fn test_interpolate_envelope_safety() {
        let mut env_a = EnvelopeParams::default();
        let mut env_b = EnvelopeParams::default();

        // Test very short times that should be clamped
        env_a.attack = 0.0;
        env_b.attack = 0.0001;

        let result = interpolate_envelope(&env_a, &env_b, 0.5);
        assert!(result.attack >= 0.001); // Should be clamped
        assert!(result.release >= 0.01); // Default + clamp
    }

    #[test]
    fn test_interpolate_filter_frequency_safety() {
        let mut fil_a = FilterParams::default();
        let mut fil_b = FilterParams::default();

        fil_a.frequency = 10.0; // Below safe range
        fil_b.frequency = 25000.0; // Above safe range

        let result = interpolate_filter(&fil_a, &fil_b, 0.5);
        assert!(result.frequency >= 20.0);
        assert!(result.frequency <= 20000.0);
    }
}
