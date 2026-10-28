//! Flanger Odin 2, mono; la chaîne possède une instance indépendante par canal.
//! `FX/Flanger.cpp:18-44`, `Flanger.h:27-124`, `Filters/CombFilter.cpp:24-73`.
//! GPL-3.0-or-later.

use super::common::interpolate;

const BUFFER_LENGTH: usize = 5280;
pub struct Flanger {
    buffer: [f32; BUFFER_LENGTH],
    write_index: usize,
    sample_rate: f32,
    delay_control: f64,
    delay_smooth: f64,
    reset_smoothing: bool,
    phase: f32,
    reset_phase: f32,
    sign: f32,
    increment: f32,
    rate: f32,
    base_time: f32,
    amount: f32,
    mix: f32,
    feedback: f32,
    sync_ratio: f32,
    modulation: [f32; 4],
}
impl Flanger {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buffer: [0.0; BUFFER_LENGTH], write_index: 0, sample_rate,
            delay_control: 1.0 / 2000.0, delay_smooth: 1.0 / 2000.0, reset_smoothing: false,
            phase: 0.0, reset_phase: 0.0, sign: 1.0, increment: 0.2 / sample_rate * 2.0,
            rate: 0.2, base_time: 0.0105, amount: 0.3, mix: 1.0,
            feedback: 0.6, sync_ratio: 3.0 / 16.0, modulation: [0.0; 4],
        }
    }
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.set_lfo_freq(self.rate);
    }
    pub fn set_lfo_freq(&mut self, frequency: f32) {
        self.rate = frequency;
        self.increment = frequency / self.sample_rate * 2.0;
    }
    pub fn set_lfo_from_bpm(&mut self, bpm: f32) { self.set_lfo_freq(bpm / self.sync_ratio / 240.0); }
    pub fn set_sync_ratio(&mut self, numerator: f32, denominator: f32) { self.sync_ratio = numerator / denominator; }
    pub fn set_base_time(&mut self, time: f32) { self.base_time = time; }
    pub fn set_amount(&mut self, amount: f32) { self.amount = amount; }
    pub fn set_dry_wet(&mut self, mix: f32) { self.mix = mix; }
    pub fn set_feedback(&mut self, feedback: f32) { self.feedback = feedback.clamp(-0.98, 0.98); }
    pub fn set_lfo_reset_pos(&mut self, phase: f32) { self.reset_phase = phase; self.reset_lfo(); }
    pub fn set_modulation(&mut self, rate: f32, amount: f32, feedback: f32, mix: f32) {
        self.modulation = [rate, amount, feedback, mix];
    }
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_index = 0;
        self.delay_smooth = self.delay_control;
        self.reset_smoothing = true;
        self.reset_lfo();
    }
    pub fn reset_lfo(&mut self) {
        self.phase = self.reset_phase;
        // Flanger.h conserve le signe courant lors du reset LFO.
    }
    pub fn process(&mut self, input: f32) -> f32 {
        let mut inc = self.increment;
        if self.modulation[0] != 0.0 {
            inc = (inc as f64 * libm::pow(4.0, (self.modulation[0] * 2.0) as f64)) as f32;
        }
        self.phase += inc;
        while self.phase > 1.0 { self.phase -= 1.0; self.sign = -self.sign; }
        let lfo = 4.0 * (self.phase * (1.0 - self.phase)) * self.sign;
        let amount = (self.amount + self.modulation[1]).clamp(0.0, 1.0);
        self.delay_control = (self.base_time + lfo * amount * 0.0095) as f64;
        if self.reset_smoothing { self.delay_smooth = self.delay_control; self.reset_smoothing = false; }
        self.delay_smooth = (self.delay_smooth - self.delay_control) * 0.999 + self.delay_control;
        let time = (self.delay_smooth as f32).min(1.0 / 40.0);
        let index = self.write_index as f32 - time * self.sample_rate;
        let delayed = interpolate(&self.buffer, index, true);
        let feedback = (self.feedback + self.modulation[2]).clamp(-0.97, 0.97);
        self.buffer[self.write_index] = input + delayed * feedback;
        self.write_index += 1;
        if self.write_index == BUFFER_LENGTH { self.write_index = 0; }
        // CombFilter.cpp appelle le bloqueur DC mais ignore son retour: le signal
        // audible est exactement (input + delayed) * 0.5, sans filtrage DC.
        let wet = (input + delayed) * 0.5;
        let mix = (self.mix + self.modulation[3]).clamp(0.0, 1.0);
        mix * wet + (1.0 - mix) * input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_lfo_preserves_upstream_polarity() {
        let mut f = Flanger::new(44100.0);
        f.sign = -1.0;
        f.reset_lfo();
        assert_eq!(f.sign, -1.0);
    }
    #[test]
    fn full_wet_comb_still_contains_half_dry_input() {
        let mut f = Flanger::new(44100.0);
        f.set_amount(0.0);
        f.reset();
        assert_eq!(f.process(1.0), 0.5);
    }
}
