//! Routage des filtres Odin (Process.cpp:257-329 et OdinFilterBase.h:47-75).

use super::{CombFilter, DiodeFilter, Filter, FormantFilter, Korg35Filter, LadderFilter,
    LadderFilterType, SemFilter, Vowel};
use crate::preset::{FilterParams, FilterType};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FilterModulation {
    pub frequency: f32,
    pub resonance: f32,
    pub gain: f32,
    pub saturation: f32,
    pub envelope: f32,
    pub velocity: f32,
    pub keyboard: f32,
    pub sem_transition: f32,
    pub formant_transition: f32,
    pub ring_amount: f32,
}

/// Tous les états sont conservés: changer un type ou un contrôle n'alloue rien.
/// Le tampon comb est alloué uniquement dans `new`.
pub struct PresetFilter {
    params: FilterParams,
    modulation: FilterModulation,
    ladder: LadderFilter,
    sem: SemFilter,
    diode: DiodeFilter,
    korg: Korg35Filter,
    formant: FormantFilter,
    comb: CombFilter,
    sample_rate: f32,
    note: f32,
    /// Vélocité MIDI brute 0..=127 (l'amont calcule `x * (float)vel / 127.f`, dans cet ordre).
    velocity: f32,
    envelope: f32,
    /// Position de lecture du sinus d'anneau, en échantillons de table (double comme l'amont).
    ring_phase: f64,
    frequency_smooth: f32,
    gain_smooth: f32,
    gain_target: f32,
    last_frequency: f32,
    last_resonance: f32,
}

impl PresetFilter {
    pub fn new(sample_rate: f32) -> Self {
        // Vowels alignés sur `FilterParams::default()`, sans quoi `configure` ne les appliquerait
        // jamais quand le preset garde la valeur par défaut (vowel_right = 2).
        let defaults = FilterParams::default();
        let mut formant = FormantFilter::new(sample_rate);
        formant.set_vowel_left(Vowel::from(defaults.vowel_left.clamp(0, 7) as u8));
        formant.set_vowel_right(Vowel::from(defaults.vowel_right.clamp(0, 7) as u8));
        Self {
            params: FilterParams::default(), modulation: FilterModulation::default(),
            ladder: LadderFilter::new(sample_rate), sem: SemFilter::new(sample_rate),
            diode: DiodeFilter::new(sample_rate), korg: Korg35Filter::new(sample_rate),
            formant, comb: CombFilter::new(sample_rate),
            sample_rate, note: 0.0, velocity: 0.0, envelope: 0.0, ring_phase: 0.0,
            frequency_smooth: 2000.0, gain_smooth: 1.0, gain_target: 1.0,
            last_frequency: -1.0, last_resonance: -1.0,
        }
    }

    pub fn configure(&mut self, params: &FilterParams) {
        if self.params.filter_type != params.filter_type {
            self.last_frequency = -1.0;
            self.last_resonance = -1.0;
            match params.filter_type {
                FilterType::LP24 => self.ladder.set_filter_type(LadderFilterType::LP4),
                FilterType::LP12 => self.ladder.set_filter_type(LadderFilterType::LP2),
                FilterType::BP24 => self.ladder.set_filter_type(LadderFilterType::BP4),
                FilterType::BP12 => self.ladder.set_filter_type(LadderFilterType::BP2),
                FilterType::HP24 => self.ladder.set_filter_type(LadderFilterType::HP4),
                FilterType::HP12 => self.ladder.set_filter_type(LadderFilterType::HP2),
                FilterType::Korg => self.korg.set_lowpass(),
                FilterType::KorgHp => self.korg.set_highpass(),
                _ => {},
            }
        }
        if self.params.vowel_left != params.vowel_left {
            self.formant.set_vowel_left(Vowel::from(params.vowel_left.clamp(0, 7) as u8));
        }
        if self.params.vowel_right != params.vowel_right {
            self.formant.set_vowel_right(Vowel::from(params.vowel_right.clamp(0, 7) as u8));
        }
        // PluginProcessorValueChange.cpp:1486 : `setPositive(!value)`, la valeur 0 est positive.
        self.comb.set_positive(!params.comb_polarity);
        self.gain_target = decibels(params.gain);
        self.params = *params;
    }

    /// Filtres de voix : `Voice::setFilterMIDIValues` décale la note de 21 (A0) et borne à zéro.
    pub fn set_midi(&mut self, note: u8, velocity: u8) {
        self.note = note.saturating_sub(21) as f32;
        self.velocity = velocity as f32;
    }

    /// Filtre 3 global : `PluginProcessorMidi.cpp:122-135` transmet la note MIDI brute, sans décalage.
    pub fn set_midi_unshifted(&mut self, note: u8, velocity: u8) {
        self.note = note as f32;
        self.velocity = velocity as f32;
    }

    pub fn set_env_value(&mut self, value: f32) { self.envelope = value; }
    pub fn set_modulation(&mut self, modulation: &FilterModulation) { self.modulation = *modulation; }
    pub fn filter_type(&self) -> FilterType { self.params.filter_type }

    /// Initialise seulement les contrôles lissés, sans effacer les mémoires DSP.
    pub fn set_smoothing_state(&mut self, frequency: f32, gain: f32) {
        self.frequency_smooth = frequency;
        self.gain_smooth = gain;
    }

    pub(crate) fn advance_controls(&mut self) {
        self.frequency_smooth = self.frequency_smooth * 0.998
            + self.params.frequency * (1.0 - 0.998);
        self.gain_smooth = self.gain_smooth * 0.995 + self.gain_target * (1.0 - 0.995);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.advance_controls();
        let p = &self.params;
        let m = self.modulation;
        if p.filter_type == FilterType::None { return input; }
        let keyboard = (p.kbd_follow + m.keyboard).max(0.0);
        let velocity = (p.vel_amount + m.velocity).max(0.0);
        let env = self.envelope * (p.env_amount + m.envelope);
        if !matches!(p.filter_type, FilterType::Comb | FilterType::Formant | FilterType::RingMod) {
        // OdinFilterBase::update() (OdinFilterBase.h:47-60): l'exposant n'est appliqué que si la
        // somme freq+kbd+env+vel est non nulle (elle peut s'annuler sans que l'exposant le fasse).
        let gate = m.frequency + keyboard + p.env_amount + m.envelope + velocity != 0.0;
        let semitones = if gate {
            m.frequency * 64.0 + self.note * keyboard
                + (env + velocity * self.velocity / 127.0) * 64.0
        } else {
            0.0
        };
        let frequency = (self.frequency_smooth * crate::dsp::math::pitch_shift_multiplier(semitones))
            .clamp(20.0, 20000.0);
        let resonance = match p.filter_type {
            FilterType::SEM12 => {
                // La modulation s'ajoute après la courbe puissance quatre, non avant.
                let q = 24.5 * p.resonance * p.resonance * p.resonance * p.resonance
                    + 0.5 + m.resonance * 24.5;
                q.clamp(0.5, 25.0)
            },
            FilterType::LP24 | FilterType::LP12 | FilterType::BP24 | FilterType::BP12
                | FilterType::HP24 | FilterType::HP12 => p.resonance + m.resonance * 4.0 / 3.88,
            FilterType::Korg | FilterType::KorgHp => p.resonance + m.resonance * 2.0 / 1.95,
            _ => p.resonance + m.resonance,
        };
        let resonance = if p.filter_type == FilterType::SEM12 { resonance }
            else { resonance.clamp(0.0, 1.0) };
        if frequency != self.last_frequency {
            match p.filter_type {
                FilterType::SEM12 => self.sem.set_cutoff(frequency),
                FilterType::Diode => self.diode.set_cutoff(frequency),
                FilterType::Korg | FilterType::KorgHp => self.korg.set_cutoff(frequency),
                _ => self.ladder.set_cutoff(frequency),
            }
            self.last_frequency = frequency;
        }
        if resonance != self.last_resonance {
            match p.filter_type {
                FilterType::SEM12 => self.sem.set_q_resonance(resonance),
                FilterType::Diode => self.diode.set_resonance(resonance),
                FilterType::Korg | FilterType::KorgHp => self.korg.set_resonance(resonance),
                _ => self.ladder.set_resonance(resonance),
            }
            self.last_resonance = resonance;
        }
        }
        let saturation = (2.0 * (p.saturation + m.saturation)).max(0.0);
        let output = match p.filter_type {
            FilterType::None => input,
            FilterType::LP24 | FilterType::LP12 | FilterType::BP24 | FilterType::BP12
                | FilterType::HP24 | FilterType::HP12 => {
                self.ladder.set_overdrive(saturation);
                self.ladder.process(input)
            },
            FilterType::SEM12 => {
                self.sem.set_transition(p.sem_transition + m.sem_transition * 2.0);
                self.sem.process(input)
            },
            FilterType::Diode => {
                self.diode.set_overdrive(saturation);
                self.diode.process(input)
            },
            FilterType::Korg | FilterType::KorgHp => {
                self.korg.set_overdrive(saturation);
                self.korg.process(input)
            },
            FilterType::Formant => {
                // FormantFilter::update() (FormantFilter.cpp:36-39) lit `m_MIDI_velocity`, mais
                // Voice::setFilterMIDIValues ne l'écrit jamais pour ce filtre: il reste à zéro
                // en amont, la vélocité ne module donc pas la transition.
                self.formant.set_transition(p.formant_transition + m.formant_transition + env);
                self.formant.process(input)
            },
            FilterType::Comb => {
                self.comb.set_frequency(self.frequency_smooth);
                self.comb.set_resonance((p.resonance + m.resonance).clamp(0.0, 1.0));
                // CombFilter.cpp:36-43 : l'exposant n'est appliqué que si l'une des deux sommes
                // (freq+vel+env, kbd) est non nulle; elles peuvent s'annuler sans que l'exposant
                // le fasse. 48 demi-tons, kbd/vel non bornés à zéro.
                let gate = m.frequency + p.vel_amount + m.velocity + p.env_amount + m.envelope != 0.0
                    || p.kbd_follow + m.keyboard != 0.0;
                let pitch = if gate {
                    (m.frequency + env + (p.vel_amount + m.velocity) * self.velocity / 127.0) * 48.0
                        + self.note * (p.kbd_follow + m.keyboard)
                } else {
                    0.0
                };
                self.comb.process_with_modulation(input, pitch)
            },
            FilterType::RingMod => {
                let pitch = m.frequency * 24.0 + self.note * keyboard
                    + (env + velocity * self.velocity / 127.0) * 48.0;
                // RingModulator::update() puis WavetableOsc1D::doWavetable(): position de lecture
                // et fréquence en double, comme l'amont (la dérive en f32 atteint 1e-4 en 1 s).
                let frequency = (self.frequency_smooth as f64 * pitch_multiplier_f64(pitch)).min(20480.0);
                let length = crate::constants::WAVETABLE_LENGTH;
                let table = crate::wavetables::get_subtable(0, 0);
                let index = self.ring_phase as usize;
                let next = if index + 1 >= length { 0 } else { index + 1 };
                let fraction = (self.ring_phase - index as f32 as f64) as f32;
                let sine = fraction * (table[next] - table[index]) + table[index];
                self.ring_phase += length as f64 * (frequency * (1.0 / self.sample_rate as f64));
                while self.ring_phase >= length as f64 {
                    self.ring_phase -= length as f64;
                }
                let amount = (p.ring_mod_amount + m.ring_amount).clamp(0.0, 1.0);
                input * (sine * amount + 1.0 - amount)
            },
        };
        output * modulated_gain(self.gain_smooth, m.gain)
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.ladder.set_sample_rate(sample_rate);
        self.sem.set_sample_rate(sample_rate);
        self.diode.set_sample_rate(sample_rate);
        self.korg.set_sample_rate(sample_rate);
        self.formant.set_sample_rate(sample_rate);
        self.comb.set_sample_rate(sample_rate);
        self.last_frequency = -1.0;
    }

    pub fn reset(&mut self) {
        self.ladder.reset(); self.sem.reset(); self.diode.reset(); self.korg.reset();
        self.formant.reset(); self.comb.reset();
        // RingModulator::reset() est intentionnellement vide en amont.
        self.last_frequency = -1.0;
    }
}

/// `juce::Decibels::decibelsToGain(db, -59.99f)`: `pow(10, db * 0.05)`, zéro à -59.99 dB et en dessous.
pub(crate) fn decibels(value: f32) -> f32 {
    if value <= -59.99 { 0.0 } else { libm::powf(10.0, value * 0.05) }
}

/// `Oscillator::pitchShiftMultiplier` (Oscillator.cpp:25-32): la Padé de juce est instanciée en
/// double (`0.05776226504 * float`), puis le résultat est arrondi en float.
fn pitch_multiplier_f64(semitones: f32) -> f64 {
    let x = 0.05776226504_f64 * semitones as f64;
    let value = if semitones < 48.0 && semitones > -48.0 {
        let numerator = 1680.0 + x * (840.0 + x * (180.0 + x * (20.0 + x)));
        let denominator = 1680.0 + x * (-840.0 + x * (180.0 + x * (-20.0 + x)));
        numerator / denominator
    } else {
        libm::exp(x)
    };
    value as f32 as f64
}

/// Même loi asymétrique pour oscillateurs, filtres et amplificateur.
pub(crate) fn modulated_gain(gain: f32, modulation: f32) -> f32 {
    if modulation < 0.0 {
        (gain * (1.0 + modulation)).max(0.0)
    } else if modulation > 0.0 {
        let value = if gain > 0.251189 {
            gain * libm::powf(3.981072, modulation)
        } else {
            gain + (1.0 - gain) * modulation
        };
        value.min(3.981072)
    } else { gain }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_bypasses_gain_and_modulation() {
        let mut filter = PresetFilter::new(44100.0);
        let params = FilterParams { filter_type: FilterType::None, gain: -60.0,
            ..FilterParams::default() };
        filter.configure(&params);
        filter.set_modulation(&FilterModulation { gain: -1.0, saturation: 1.0,
            ..FilterModulation::default() });
        assert_eq!(filter.process(0.75), 0.75);
        assert_eq!(filter.process(-0.25), -0.25);
    }

    #[test]
    fn keyboard_origin_and_envelope_have_the_same_pitch_scale() {
        // 12 touches au-dessus de A0 équivalent à 12/64 d'enveloppe.
        let params = FilterParams { filter_type: FilterType::LP24, frequency: 1000.0,
            resonance: 0.2, kbd_follow: 1.0, ..FilterParams::default() };
        let mut keyboard = PresetFilter::new(44100.0);
        keyboard.configure(&params);
        keyboard.set_midi(33, 127);
        let mut envelope = PresetFilter::new(44100.0);
        envelope.configure(&FilterParams { kbd_follow: 0.0, env_amount: 12.0 / 64.0, ..params });
        envelope.set_midi(21, 127);
        envelope.set_env_value(1.0);
        for i in 0..512 {
            let input = if i == 0 { 1.0 } else { 0.0 };
            assert!((keyboard.process(input) - envelope.process(input)).abs() < 1e-6);
        }
    }

    #[test]
    fn reconfiguring_a_held_filter_preserves_its_tail() {
        let params = FilterParams { filter_type: FilterType::LP24, frequency: 300.0,
            resonance: 0.8, ..FilterParams::default() };
        let mut updated = PresetFilter::new(44100.0);
        let mut uninterrupted = PresetFilter::new(44100.0);
        updated.configure(&params);
        uninterrupted.configure(&params);
        for i in 0..128 {
            let input = if i == 0 { 1.0 } else { 0.0 };
            updated.process(input);
            uninterrupted.process(input);
        }
        updated.configure(&params);
        for _ in 0..128 {
            assert_eq!(updated.process(0.0), uninterrupted.process(0.0));
        }
    }
}
