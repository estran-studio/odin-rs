//! Chorus Odin 2: `Source/audio/FX/Chorus.cpp:25-120`, `Chorus.h:24-151`.
//! Les deux canaux mono indépendants suivent PluginProcessorMisc.cpp:230-232.
//! GPL-3.0-or-later.

extern crate alloc;
use alloc::boxed::Box;
use super::common::{buffer, interpolate, Allpass};

struct Channel {
    buffer: Box<[f32]>,
    write_index: usize,
    sample_rate: f32,
    phase: f32,
    reset_phase: f32,
    increment: f32,
    rate: f32,
    amount: f32,
    amount_control: f32,
    mix: f32,
    feedback: f32,
    allpass: [Allpass; 2],
    modulation: [f32; 4],
}
impl Channel {
    fn new(sample_rate: f32, reset_phase: f32) -> Self {
        let mut channel = Self {
            buffer: buffer(44100), write_index: 0, sample_rate, phase: reset_phase,
            reset_phase, increment: 0.0, rate: 0.15, amount: 0.16, amount_control: 0.16,
            mix: 1.0, feedback: 0.0,
            allpass: [Allpass::new(sample_rate, 1.25), Allpass::new(sample_rate, 1.3)],
            modulation: [0.0; 4],
        };
        channel.set_sample_rate(sample_rate);
        channel
    }
    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
        self.increment = 2.0 * self.rate / rate;
        self.allpass[0].set_sample_rate(rate);
        self.allpass[0].set_frequency(1000.0);
        self.allpass[1].set_sample_rate(rate);
        self.allpass[1].set_frequency(5000.0);
    }
    fn reset(&mut self) {
        self.buffer.fill(0.0);
        for a in &mut self.allpass { a.reset(); }
        self.phase = 0.0;
        self.amount = self.amount_control;
        // Chorus.h conserve write_index lors de reset.
    }
    fn process(&mut self, input: f32) -> f32 {
        self.amount = ((self.amount - self.amount_control) as f64 * 0.995 + self.amount_control as f64) as f32;
        let mut inc = self.increment;
        if self.modulation[0] != 0.0 {
            inc = (inc as f64 * libm::pow(4.0, (self.modulation[0] * 2.0) as f64)) as f32;
        }
        self.phase += inc;
        while self.phase > 2.0 { self.phase -= 2.0; }
        let sign = if self.phase < 1.0 { 1.0 } else { -1.0 };
        let pos = if self.phase > 1.0 { self.phase - 1.0 } else { self.phase };
        let lfo = 4.0 * (pos * (1.0 - pos)) * sign * 0.5 + 0.5;
        let amount = (self.amount + self.modulation[1]).max(0.0);
        // Constantes double dans le C++ amont.
        let read1 = (self.write_index as f64 - (0.015 + lfo as f64 * 0.02 * amount as f64) * self.sample_rate as f64) as f32;
        let tap1 = interpolate(&self.buffer, read1, true);
        let ap2 = self.allpass[1].process(tap1 as f64);
        let mut output = self.allpass[0].process(ap2) as f32;
        let read2 = (self.write_index as f64 - (0.011 + (1.0 - lfo) as f64 * 0.02 * amount as f64) * self.sample_rate as f64) as f32;
        output -= interpolate(&self.buffer, read2, true);
        output = (output as f64 + input as f64 * 0.5) as f32;
        output *= 0.4;
        let feedback = (self.feedback + self.modulation[2]).clamp(-1.0, 1.0);
        self.buffer[self.write_index] = input + output * feedback;
        self.write_index += 1;
        if self.write_index == self.buffer.len() { self.write_index = 0; }
        let mix = (self.mix + self.modulation[3]).clamp(0.0, 1.0);
        output * mix + (1.0 - mix) * input
    }
}

pub struct Chorus {
    channel: [Channel; 2],
    sync_ratio: f32,
}
impl Chorus {
    pub fn new(sample_rate: f32) -> Self {
        Self { channel: [Channel::new(sample_rate, 0.0), Channel::new(sample_rate, 0.25)], sync_ratio: 3.0 / 16.0 }
    }
    pub fn set_lfo_freq(&mut self, frequency: f32) {
        for c in &mut self.channel { c.rate = frequency; c.increment = 2.0 * frequency / c.sample_rate; }
    }
    pub fn set_lfo_from_bpm(&mut self, bpm: f32) { self.set_lfo_freq(bpm / self.sync_ratio / 240.0); }
    pub fn set_sync_ratio(&mut self, numerator: f32, denominator: f32) { self.sync_ratio = numerator / denominator; }
    pub fn set_amount(&mut self, amount: f32) { for c in &mut self.channel { c.amount_control = amount * amount; } }
    pub fn set_dry_wet(&mut self, mix: f32) { for c in &mut self.channel { c.mix = mix; } }
    pub fn set_feedback(&mut self, feedback: f32) { for c in &mut self.channel { c.feedback = feedback; } }
    pub fn set_modulation(&mut self, rate: f32, amount: f32, feedback: f32, mix: f32) {
        for c in &mut self.channel { c.modulation = [rate, amount, feedback, mix]; }
    }
    pub fn set_sample_rate(&mut self, sample_rate: f32) { for c in &mut self.channel { c.set_sample_rate(sample_rate); } }
    pub fn reset(&mut self) { for c in &mut self.channel { c.reset(); } }
    pub fn reset_lfo(&mut self) { for c in &mut self.channel { c.phase = c.reset_phase; } }
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        (self.channel[0].process(left), self.channel[1].process(right))
    }
    pub fn process(&mut self, input: f32) -> (f32, f32) { self.process_stereo(input, input) }
    pub fn process_mono(&mut self, input: f32) -> f32 {
        let (left, right) = self.process(input);
        (left + right) * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dry_mix_preserves_independent_input_channels() {
        let mut c = Chorus::new(44100.0);
        c.set_dry_wet(0.0);
        assert_eq!(c.process_stereo(0.5, -0.25), (0.5, -0.25));
    }
    #[test]
    fn negative_feedback_changes_the_audible_delay_response() {
        let mut negative = Chorus::new(44100.0);
        let mut zero = Chorus::new(44100.0);
        negative.set_feedback(-0.8);
        zero.set_feedback(0.0);
        negative.process(1.0);
        zero.process(1.0);
        let mut difference = 0.0_f32;
        for _ in 0..2000 {
            difference += (negative.process(0.0).0 - zero.process(0.0).0).abs();
        }
        assert!(difference > 0.01);
    }
}
