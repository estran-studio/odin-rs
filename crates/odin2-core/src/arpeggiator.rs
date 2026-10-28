//! Séquenceur traduit de `Source/gui/OdinArpeggiator.cpp`.
//! Les séquences sont indexées sans vector temporaire, même pour Crawl.

use crate::preset::ArpeggiatorParams;

#[derive(Clone, Copy, Default)]
pub(crate) struct ArpNote {
    pub note: u8,
    pub velocity: u8,
    pub modulation: [f32; 2],
}

#[derive(Clone, Copy)]
struct PendingOff { note: u8, time: f64 }

pub(crate) struct Arpeggiator {
    velocity: [u8; 128],
    sustained: [bool; 128],
    sustain: bool,
    keys: [u8; 128],
    key_count: usize,
    random: [usize; 1024],
    rng: u32,
    index: usize,
    step: usize,
    start: bool,
    ended: bool,
    elapsed: f64,
    pending: [PendingOff; 128],
    pending_count: usize,
}

impl Arpeggiator {
    pub fn new() -> Self {
        Self {
            velocity: [0; 128], sustained: [false; 128], sustain: false,
            keys: [0; 128], key_count: 0, random: [0; 1024], rng: 0x4f64696e,
            index: 0, step: 0, start: false, ended: false, elapsed: 0.0,
            pending: [PendingOff { note: 0, time: 0.0 }; 128], pending_count: 0,
        }
    }

    fn rebuild(&mut self, params: &ArpeggiatorParams) {
        self.key_count = 0;
        for note in 0..128 {
            if self.velocity[note] != 0 {
                self.keys[self.key_count] = note as u8;
                self.key_count += 1;
            }
        }
        if self.key_count == 0 {
            for off in &mut self.pending[..self.pending_count] { off.time = f64::MAX; }
        }
        let count = self.key_count * params.octaves.clamp(1, 8) as usize;
        for (i, slot) in self.random.iter_mut().enumerate().take(count) { *slot = i; }
        // `std::random_device` d'Odin est remplacé par une graine fixe : rendu reproductible.
        if params.direction as u8 == 50 {
            for i in (1..count).rev() {
                self.rng ^= self.rng << 13;
                self.rng ^= self.rng >> 17;
                self.rng ^= self.rng << 5;
                self.random.swap(i, self.rng as usize % (i + 1));
            }
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, params: &ArpeggiatorParams) {
        if note >= 128 { return; }
        self.sustained[note as usize] = false;
        if self.sustain && self.velocity[note as usize] != 0 { return; }
        if self.key_count == 0 { self.start = true; }
        self.velocity[note as usize] = velocity;
        self.ended = false;
        self.rebuild(params);
    }

    pub fn note_off(&mut self, note: u8, params: &ArpeggiatorParams) {
        if note >= 128 { return; }
        if self.sustain { self.sustained[note as usize] = true; }
        else { self.velocity[note as usize] = 0; }
        self.rebuild(params);
    }

    pub fn set_sustain(&mut self, sustain: bool, params: &ArpeggiatorParams) {
        self.sustain = sustain;
        if !sustain {
            for note in 0..128 {
                if self.sustained[note] { self.velocity[note] = 0; }
            }
            self.sustained.fill(false);
            self.rebuild(params);
        }
    }

    pub fn reset(&mut self) {
        self.velocity.fill(0);
        self.sustained.fill(false);
        self.sustain = false;
        self.key_count = 0;
        self.pending_count = 0;
        self.start = false;
        self.ended = false;
        self.index = 0;
        self.step = 0;
        self.elapsed = 0.0;
        self.rng = 0x4f64696e;
    }

    fn sequence_length(&self, params: &ArpeggiatorParams) -> usize {
        let n = self.key_count * params.octaves.clamp(1, 8) as usize;
        match params.direction as u8 {
            10 | 20 | 50 => n,
            30 | 40 => n.saturating_mul(2).saturating_sub(1),
            60 | 70 => (n.saturating_sub(self.key_count) + 1) * self.key_count,
            _ => 0, // CrawlUpDown/DownUp sont aussi absents du switch C++.
        }
    }

    pub fn tick(
        &mut self, params: &ArpeggiatorParams, sample_rate: f32, bpm: f32,
        sync_ratio: f32, speed_mod: f32, gate_mod: f32,
    ) -> (Option<ArpNote>, [u8; 128], usize) {
        let dt = f64::from(libm::expf(core::f32::consts::LN_2 * speed_mod) / sample_rate);
        let period = f64::from(sync_ratio * 240.0 / bpm);
        let length = self.sequence_length(params);
        let mut on = None;
        if length > 0 && !self.ended {
            self.elapsed += dt;
            let next = self.start || self.elapsed > period;
            if next {
                if self.start {
                    self.index = 0;
                    self.step = 0;
                    self.start = false;
                } else {
                    self.index += 1;
                    if self.index >= length {
                        self.index = if matches!(params.direction as u8, 30 | 40) && length > 1 { 1 } else { 0 };
                        if params.direction as u8 == 50 { self.rebuild(params); }
                    }
                    self.step += 1;
                    if self.step >= params.steps.clamp(1, 16) as usize {
                        self.step = 0;
                        self.ended = params.one_shot;
                    }
                }
                self.elapsed = 0.0;
                let step = &params.step_params[self.step];
                if !self.ended && step.on {
                    let n = self.key_count * params.octaves.clamp(1, 8) as usize;
                    let i = self.index % length;
                    let linear = match params.direction as u8 {
                        20 => n - 1 - i,
                        30 => if i < n { i } else { 2 * n - 2 - i },
                        40 => if i < n { n - 1 - i } else { i - n + 1 },
                        50 => self.random[i],
                        60 => i / self.key_count + i % self.key_count,
                        70 => n - 1 - (i / self.key_count + i % self.key_count),
                        _ => i,
                    };
                    let key = self.keys[linear % self.key_count];
                    let note = i32::from(key) + 12 * (linear / self.key_count) as i32 + step.transpose;
                    // L'API MIDI n'a pas de note hors de [0,127].
                    if (0..128).contains(&note) && self.pending_count < self.pending.len() {
                        let note = note as u8;
                        on = Some(ArpNote { note, velocity: self.velocity[key as usize], modulation: [step.mod1, step.mod2] });
                        self.pending[self.pending_count] = PendingOff { note, time: 0.0 };
                        self.pending_count += 1;
                    }
                }
            }
        }
        let gate = params.gate as f32 / 100.0;
        let gate = if gate_mod > 0.0 { gate + gate_mod } else { (gate + gate_mod * gate).max(0.0) };
        let mut off = [0; 128];
        let mut off_count = 0;
        let mut retained = 0;
        for i in 0..self.pending_count {
            let mut pending = self.pending[i];
            pending.time += dt;
            if pending.time > period * f64::from(gate) {
                off[off_count] = pending.note;
                off_count += 1;
            } else {
                self.pending[retained] = pending;
                retained += 1;
            }
        }
        self.pending_count = retained;
        (on, off, off_count)
    }
}
