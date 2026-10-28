//! Oscillateur analogique Odin 2: tables FatSaw/Triangle/Sine et PWM par différence de scies.
//! Le moteur DSP est commun avec PresetOscillator (AnalogOscillator.cpp:30–97).
use super::{Oscillator, OscillatorModulation, PresetOscillator, Waveform};
use crate::preset::{AnalogWaveform, OscillatorParams, OscillatorType};

#[derive(Clone)]
pub struct AnalogOscillator {
    runtime: PresetOscillator,
    params: OscillatorParams,
    modulation: OscillatorModulation,
}
impl AnalogOscillator {
    pub fn new(sample_rate: f32) -> Self {
        let params = OscillatorParams { osc_type: OscillatorType::Analog, ..Default::default() };
        let mut runtime = PresetOscillator::new(sample_rate);
        runtime.configure(&params);
        Self { runtime, params, modulation: OscillatorModulation::default() }
    }
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.params.analog_wave = match waveform {
            Waveform::Saw => AnalogWaveform::Saw, Waveform::Square => AnalogWaveform::Pulse,
            Waveform::Triangle => AnalogWaveform::Triangle, Waveform::Sine => AnalogWaveform::Sine,
        };
        self.runtime.configure(&self.params);
    }
    pub fn set_pulse_width(&mut self, pulse_width: f32) {
        self.params.pulsewidth = pulse_width; self.runtime.configure(&self.params);
    }
    pub fn set_pwm_mod(&mut self, modulation: f32) {
        self.modulation.pulse_width = modulation; self.runtime.set_modulation(self.modulation);
    }
    pub fn set_drift(&mut self, drift: f32) {
        self.params.drift = drift; self.runtime.configure(&self.params);
    }
}
impl Oscillator for AnalogOscillator {
    fn process(&mut self) -> f32 { self.runtime.process() }
    fn set_frequency(&mut self, frequency: f32) { self.runtime.set_frequency(frequency); }
    fn set_sample_rate(&mut self, sample_rate: f32) { self.runtime.set_sample_rate(sample_rate); }
    fn reset(&mut self) { self.runtime.reset(); }
    fn randomize_phase(&mut self) { self.runtime.randomize_phase(); }
}
