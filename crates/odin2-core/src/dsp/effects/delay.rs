//! Delay stéréo Odin 2: `Source/audio/FX/Delay.cpp:30-150`, `Delay.h:25-165`.
//! GPL-3.0-or-later. Mémoire réservée à la construction, jamais dans les setters.

extern crate alloc;
use alloc::boxed::Box;
use super::common::{buffer, interpolate, DcBlock, Highpass};

const BUFFER_SIZE: usize = 192000 * 4;

pub struct Delay {
    buffer_left: Box<[f32]>,
    buffer_right: Box<[f32]>,
    write_index: usize,
    sample_rate: f32,
    delay_time_control: f32,
    delay_time_smooth: f32,
    feedback: f32,
    dry: f32,
    wet: f32,
    ducking: f32,
    average_sum: f32,
    average_mean: f32,
    highpass: [Highpass; 2],
    dc: [DcBlock; 2],
    ping_pong: bool,
    sync_ratio: f32,
    modulation: [f32; 4],
}
impl Delay {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buffer_left: buffer(BUFFER_SIZE), buffer_right: buffer(BUFFER_SIZE),
            write_index: 0, sample_rate, delay_time_control: 1.0, delay_time_smooth: 1.0,
            feedback: 0.45, dry: 1.0, wet: 0.7, ducking: 0.0,
            average_sum: 0.0, average_mean: 0.0,
            highpass: [Highpass::new(sample_rate), Highpass::new(sample_rate)],
            dc: [DcBlock::new(sample_rate), DcBlock::new(sample_rate)],
            ping_pong: false, sync_ratio: 3.0 / 16.0, modulation: [0.0; 4],
        }
    }
    pub fn set_delay_time(&mut self, time: f32) { self.delay_time_control = time; }
    pub fn set_delay_from_bpm(&mut self, bpm: f32) { self.set_delay_time(240.0 * self.sync_ratio / bpm); }
    pub fn set_sync_ratio(&mut self, numerator: f32, denominator: f32) { self.sync_ratio = numerator / denominator; }
    pub fn set_feedback(&mut self, feedback: f32) { self.feedback = feedback; }
    pub fn set_dry(&mut self, dry: f32) { self.dry = dry; }
    pub fn set_wet(&mut self, wet: f32) { self.wet = wet; }
    pub fn set_highpass_freq(&mut self, freq: f32) {
        self.highpass[0].set_frequency(freq);
        self.highpass[1].set_frequency(freq);
    }
    pub fn set_ducking(&mut self, amount: f32) { self.ducking = amount; }
    pub fn set_ping_pong(&mut self, enabled: bool) { self.ping_pong = enabled; }
    pub fn set_modulation(&mut self, time: f32, feedback: f32, hp: f32, dry: f32, wet: f32) {
        self.modulation = [time, feedback, dry, wet];
        self.highpass[0].set_modulation(hp);
        self.highpass[1].set_modulation(hp);
    }
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for h in &mut self.highpass { h.set_sample_rate(sample_rate); }
        for d in &mut self.dc { d.set_sample_rate(sample_rate); }
    }
    pub fn reset(&mut self) {
        self.buffer_left.fill(0.0);
        self.buffer_right.fill(0.0);
        self.write_index = 0;
        self.delay_time_smooth = self.delay_time_control;
        for h in &mut self.highpass { h.reset(); }
        for d in &mut self.dc { d.reset(); }
        // L'accumulateur de ducking n'est pas remis à zéro dans Delay.h.
    }
    fn average(&mut self, input: f32) -> f32 {
        self.average_sum += libm::fabsf(input) - self.average_mean;
        self.average_mean = self.average_sum / 441.0;
        (self.average_mean * 5.0).min(1.0)
    }
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        self.delay_time_smooth = ((self.delay_time_smooth - self.delay_time_control) as f64 * 0.99994
            + self.delay_time_control as f64) as f32;
        let mut time = self.delay_time_smooth;
        if self.modulation[0] != 0.0 { time = (time as f64 * libm::pow(3.0, self.modulation[0] as f64)) as f32; }
        let index = self.write_index as f32 - time * self.sample_rate;
        // Le delay amont tronque vers zéro (contrairement au chorus et au comb).
        let delayed_l = interpolate(&self.buffer_left, index, false);
        let feedback = (self.feedback + self.modulation[1]).clamp(0.0, 1.0);
        if self.ping_pong {
            self.buffer_left[self.write_index] = left / 2.0;
            self.buffer_right[self.write_index] = delayed_l * feedback;
        } else {
            self.buffer_left[self.write_index] = left + delayed_l * feedback;
        }
        // L'amont lit le canal droit après l'écriture du canal gauche (ping-pong).
        let delayed_r = interpolate(&self.buffer_right, index, false);
        if self.ping_pong {
            self.buffer_left[self.write_index] += right / 2.0 + delayed_r * feedback;
        } else {
            self.buffer_right[self.write_index] = right + delayed_r * feedback;
        }
        self.write_index += 1;
        if self.write_index == BUFFER_SIZE { self.write_index = 0; }
        let hp_l = self.highpass[0].process(delayed_l);
        let duck_l = self.average(left);
        let wet_l = self.dc[0].process(hp_l * (1.0 - duck_l * self.ducking));
        let hp_r = self.highpass[1].process(delayed_r);
        let duck_r = self.average(right);
        let wet_r = self.dc[1].process(hp_r * (1.0 - duck_r * self.ducking));
        let dry = (self.dry + self.modulation[2]).clamp(0.0, 1.0);
        let wet = (self.wet + self.modulation[3]).clamp(0.0, 1.0);
        (wet_l * wet + left * dry, wet_r * wet + right * dry)
    }
    pub fn process_mono(&mut self, input: f32) -> (f32, f32) { self.process(input, input) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ping_pong_sums_both_inputs_into_first_line() {
        let mut d = Delay::new(44100.0);
        d.set_ping_pong(true);
        d.set_delay_time(0.1);
        d.reset();
        d.process(0.4, 0.8);
        assert_eq!(d.buffer_left[0], 0.6);
        assert_eq!(d.buffer_right[0], 0.0);
    }
    #[test]
    fn delay_supports_four_seconds_at_192khz() {
        let mut d = Delay::new(192000.0);
        d.set_delay_time(4.0);
        d.set_dry(0.0);
        d.set_wet(1.0);
        d.set_feedback(0.0);
        d.reset();
        assert_eq!(d.process(1.0, 0.0), (0.0, 0.0));
        for _ in 1..768000 { assert_eq!(d.process(0.0, 0.0), (0.0, 0.0)); }
        let (left, right) = d.process(0.0, 0.0);
        assert!(left > 0.9);
        assert_eq!(right, 0.0);
    }
}
