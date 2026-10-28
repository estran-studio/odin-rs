//! SEM (State Variable) Filter
//!
//! An Oberheim SEM-style 12dB/oct state variable filter with morphable response.
//! Can smoothly transition between lowpass, bandpass, notch, and highpass modes.

use super::Filter;
use crate::dsp::math::fast_tan;

/// Filter frequency limits
const FILTER_FC_MIN: f32 = 20.0;
const FILTER_FC_MAX: f32 = 20000.0;

/// SEM filter mode transition values
///
/// The transition parameter controls the filter response:
/// - -1.0: Pure lowpass
/// -  0.0: Band stop (notch)
/// - +1.0: Pure highpass
///
/// Values in between create mixed responses.
pub struct SemFilter {
    /// Sample rate
    sample_rate: f32,

    /// One over sample rate
    one_over_sample_rate: f32,

    /// Cutoff frequency
    cutoff: f32,

    /// Resonance (Q)
    resonance: f32,

    /// Transition parameter (-1 = LP, 0 = BS, 1 = HP)
    transition: f32,

    /// Filter state z1
    z1: f32,

    /// Filter state z2
    z2: f32,

    /// Alpha coefficient
    alpha: f32,

    /// Alpha0 coefficient
    alpha0: f32,

    /// Rho coefficient
    rho: f32,

    /// Last frequency for optimization
    last_freq: f32,

    /// Resonance modulated
    resonance_modded: f32,
}

impl SemFilter {
    /// Create a new SEM filter
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            sample_rate,
            one_over_sample_rate: 1.0 / sample_rate,
            cutoff: 1000.0,
            resonance: 0.5,
            transition: -1.0, // Default to lowpass
            z1: 0.0,
            z2: 0.0,
            alpha: 1.0,
            alpha0: 1.0,
            rho: 1.0,
            last_freq: -1.0,
            resonance_modded: 0.5,
        };
        filter.update_coefficients();
        filter
    }

    /// Set the filter transition (-1.0 = LP, 0.0 = Notch, 1.0 = HP)
    pub fn set_transition(&mut self, transition: f32) {
        self.transition = transition.clamp(-1.0, 1.0);
    }

    /// Get the current transition value
    pub fn transition(&self) -> f32 {
        self.transition
    }

    /// Q déjà transformé: la modulation s'ajoute après la courbe puissance quatre.
    pub fn set_q_resonance(&mut self, q: f32) {
        self.resonance = q.clamp(0.5, 25.0);
        self.last_freq = -1.0;
    }

    /// Update filter coefficients
    fn update_coefficients(&mut self) {
        let freq = self.cutoff.clamp(FILTER_FC_MIN, FILTER_FC_MAX);

        if freq == self.last_freq {
            return;
        }
        self.last_freq = freq;

        // Calculate alpha using bilinear transform
        let wd = 2.0 * core::f32::consts::PI * freq;
        let wa = 2.0 * self.sample_rate * fast_tan(wd * self.one_over_sample_rate * 0.5);
        let g = wa * self.one_over_sample_rate * 0.5;

        self.resonance_modded = self.resonance.clamp(0.5, 25.0);

        let r = 1.0 / (2.0 * self.resonance_modded);

        self.alpha0 = 1.0 / (1.0 + 2.0 * r * g + g * g);
        self.alpha = g;
        self.rho = 2.0 * r + g;
    }
}

impl Filter for SemFilter {
    fn process(&mut self, input: f32) -> f32 {
        self.update_coefficients();

        // Calculate filter outputs
        let hpf = self.alpha0 * (input - self.rho * self.z1 - self.z2);
        let bpf = self.alpha * hpf + self.z1;
        let lpf = self.alpha * bpf + self.z2;

        let r = 1.0 / (2.0 * self.resonance_modded);
        let bsf = input - 2.0 * r * bpf; // Band stop (notch)

        // Update state
        self.z1 = self.alpha * hpf + bpf;
        self.z2 = self.alpha * bpf + lpf;

        // Morph between filter types based on transition
        if self.transition < 0.0 {
            // Transition from LP (-1) to BS (0)
            (1.0 + self.transition) * bsf - self.transition * lpf
        } else {
            // Transition from BS (0) to HP (1)
            self.transition * hpf + (1.0 - self.transition) * bsf
        }
    }

    fn set_cutoff(&mut self, freq: f32) {
        self.cutoff = freq.clamp(FILTER_FC_MIN, FILTER_FC_MAX);
        self.last_freq = -1.0; // Force recalculation
    }

    fn set_resonance(&mut self, res: f32) {
        let res = res.clamp(0.0, 1.0);
        self.set_q_resonance(24.5 * res * res * res * res + 0.5);
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.one_over_sample_rate = 1.0 / sample_rate;
        self.last_freq = -1.0; // Force recalculation
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sem_basic() {
        let mut filter = SemFilter::new(44100.0);
        filter.set_cutoff(1000.0);
        filter.set_resonance(0.5);

        for i in 0..1000 {
            let input = if i % 100 < 50 { 1.0 } else { -1.0 };
            let output = filter.process(input);
            assert!(output.is_finite(), "Filter produced invalid output");
        }
    }

    #[test]
    fn test_sem_transitions() {
        let mut filter = SemFilter::new(44100.0);
        filter.set_cutoff(1000.0);
        filter.set_resonance(0.5);

        // Test each transition position
        for transition in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            filter.set_transition(transition);
            filter.reset();

            for _ in 0..100 {
                let output = filter.process(0.5);
                assert!(output.is_finite(), "Transition {} produced invalid output", transition);
            }
        }
    }

    #[test]
    fn test_sem_lowpass_vs_highpass() {
        // Generate test signal (mix of low and high frequencies)
        let sample_rate = 44100.0;
        let cutoff = 1000.0;

        // High frequency (above cutoff)
        let high_freq = 5000.0;

        let mut filter_lp = SemFilter::new(sample_rate);
        filter_lp.set_cutoff(cutoff);
        filter_lp.set_transition(-1.0); // Lowpass

        let mut filter_hp = SemFilter::new(sample_rate);
        filter_hp.set_cutoff(cutoff);
        filter_hp.set_transition(1.0); // Highpass

        // Process high frequency through both
        let mut lp_energy = 0.0f32;
        let mut hp_energy = 0.0f32;

        for i in 0..1000 {
            let t = i as f32 / sample_rate;
            let input = libm::sinf(2.0 * core::f32::consts::PI * high_freq * t);

            let lp_out = filter_lp.process(input);
            let hp_out = filter_hp.process(input);

            lp_energy += lp_out * lp_out;
            hp_energy += hp_out * hp_out;
        }

        // HP should pass more of the high frequency than LP
        assert!(hp_energy > lp_energy * 0.5, "HP should pass high frequencies better");
    }
}
