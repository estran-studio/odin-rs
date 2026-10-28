//! Phaser stéréo Odin 2: `FX/Phaser.cpp:20-159`, `Phaser.h:29-237`.
//! GPL-3.0-or-later. Douze biquads double précision par canal.

use super::common::Allpass;

const OFFSETS: [f32; 12] = [-0.4, 0.0, 0.2, 0.4, -0.13, -0.34, -0.375, -0.23, 0.13, 0.36, -0.09, -0.29];
const ORDER: [usize; 12] = [2, 1, 0, 5, 4, 3, 8, 7, 6, 11, 10, 9];
pub struct Phaser {
    allpass: [[Allpass; 12]; 2],
    sample_rate: f32,
    phase: [f32; 2],
    sign: [f32; 2],
    stored: [f32; 2],
    increment: f32,
    rate: f32,
    base_frequency: f32,
    amount: f32,
    mix: f32,
    feedback: f32,
    width: f32,
    sync_ratio: f32,
    modulation: [f32; 5],
}
impl Phaser {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            allpass: [[Allpass::new(sample_rate, 1.32); 12]; 2], sample_rate,
            phase: [0.0, 0.5], sign: [1.0; 2], stored: [0.0; 2],
            increment: 2.0 * 0.25 / sample_rate, rate: 0.25, base_frequency: 4000.0,
            amount: 0.3, mix: 0.5, feedback: 0.25 * 0.97, width: 1.0,
            sync_ratio: 3.0 / 16.0, modulation: [0.0; 5],
        }
    }
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.set_lfo_freq(self.rate);
        for channel in &mut self.allpass { for a in channel { a.set_sample_rate(sample_rate); } }
    }
    pub fn set_lfo_freq(&mut self, frequency: f32) {
        self.rate = frequency;
        self.increment = 2.0 * frequency / self.sample_rate;
    }
    pub fn set_lfo_from_bpm(&mut self, bpm: f32) { self.set_lfo_freq(bpm / self.sync_ratio / 240.0); }
    pub fn set_sync_ratio(&mut self, numerator: f32, denominator: f32) { self.sync_ratio = numerator / denominator; }
    pub fn set_base_freq(&mut self, frequency: f32) { self.base_frequency = frequency; }
    pub fn set_amount(&mut self, amount: f32) { self.amount = amount; }
    pub fn set_dry_wet(&mut self, mix: f32) { self.mix = mix * 0.5; }
    pub fn set_feedback(&mut self, feedback: f32) { self.feedback = (feedback as f64 * 0.97) as f32; }
    pub fn set_radius(&mut self, radius: f32) {
        for channel in &mut self.allpass { for a in channel { a.set_radius(radius); } }
    }
    pub fn set_width(&mut self, width: f32) { self.width = width; }
    pub fn set_modulation(&mut self, rate: f32, amount: f32, frequency: f32, feedback: f32, mix: f32) {
        self.modulation = [rate, amount, frequency, feedback, mix];
    }
    pub fn reset(&mut self) {
        for channel in &mut self.allpass { for a in channel { a.reset(); } }
        self.phase = [0.0; 2];
        self.stored = [0.0; 2];
        // Comme Phaser.h, ne réinitialise pas les signes.
    }
    pub fn reset_lfo(&mut self) { self.phase = [0.0, 0.5]; }
    fn process_channel(&mut self, input: f32, channel: usize) -> f32 {
        let feedback = (self.feedback + self.modulation[3]).clamp(0.0, 0.97);
        let with_feedback = input + self.stored[channel] * feedback;
        let mut inc = self.increment;
        if self.modulation[0] != 0.0 {
            // Asymétrie réelle de Phaser.h: gauche pow(4,m), droite pow(4,2*m).
            let exponent = if channel == 0 { self.modulation[0] } else { self.modulation[0] * 2.0 };
            inc = (inc as f64 * libm::pow(4.0, exponent as f64)) as f32;
        }
        self.phase[channel] += inc;
        while self.phase[channel] > 1.0 { self.phase[channel] -= 1.0; self.sign[channel] = -self.sign[channel]; }
        let pos = self.phase[channel];
        let lfo = 4.0 * (pos * (1.0 - pos)) * self.sign[channel];
        let amount = (self.amount + self.modulation[1]).clamp(0.0, 1.5);
        let base = (self.base_frequency + self.modulation[2] * 2000.0).clamp(400.0, 8000.0);
        let frequency = base + lfo * amount * 4000.0;
        for (i, offset) in OFFSETS.iter().enumerate() {
            self.allpass[channel][i].set_frequency(frequency * (1.0 + offset * self.width));
        }
        let mut shifted = with_feedback as f64;
        for stage in ORDER { shifted = self.allpass[channel][stage].process(shifted); }
        self.stored[channel] = shifted as f32;
        let mix = (self.mix + self.modulation[4] * 0.5).clamp(0.0, 0.5);
        (((1.0 - mix) * input) as f64 + mix as f64 * shifted) as f32
    }
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        (self.process_channel(left, 0), self.process_channel(right, 1))
    }
    pub fn process_mono(&mut self, input: f32) -> (f32, f32) { self.process(input, input) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modulation_retains_upstream_rate_asymmetry() {
        let mut p = Phaser::new(48000.0);
        p.reset_lfo();
        p.set_modulation(0.5, 0.0, 0.0, 0.0, 0.0);
        p.process(0.0, 0.0);
        assert!((p.phase[0] - 2.0 * p.increment).abs() < 1e-7);
        assert!((p.phase[1] - 0.5 - 4.0 * p.increment).abs() < 1e-7);
    }
    #[test]
    fn full_wet_parameter_remains_half_mix() {
        let mut p = Phaser::new(44100.0);
        p.set_dry_wet(1.0);
        p.set_amount(0.0);
        p.set_lfo_freq(0.0);
        let pole_gain = (1.0_f32 / (1.32_f32 * 1.32_f32)) as f64;
        let expected = 0.5 + 0.5 * libm::pow(pole_gain, 12.0);
        assert!((p.process(1.0, 0.0).0 as f64 - expected).abs() < 1e-6);
    }
}
