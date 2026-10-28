//! Réverbération Zita utilisée par Odin 2, GPL-3.0-or-later.
//!
//! `FX/ZitaReverb.cpp:141-461`, `ZitaReverb.h:42-147`, `Filters/BiquadEQ.cpp:42-59`.
//! Réseau huit voies, diffuseurs allpass et EQ double précision. Les buffers pour
//! 192 kHz sont réservés dans new; changer les paramètres ou la fréquence n'alloue pas.

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;
use super::common::MAX_SAMPLE_RATE;

/// Default EQ Q for reverb
const REVERB_EQ_Q_DEFAULT: f32 = 0.9;

/// BiquadEQ.cpp:42-59 et BiquadFilter.cpp:32-44: coefficients/états double.
struct OutputEq {
    sample_rate: f32,
    frequency: f32,
    gain: f32,
    a1: f64,
    a2: f64,
    b0: f64,
    b2: f64,
    z1: f64,
    z2: f64,
}
impl OutputEq {
    fn new(sample_rate: f32) -> Self {
        let mut eq = Self { sample_rate, frequency: 1000.0, gain: 0.0,
            a1: 0.0, a2: 0.0, b0: 1.0, b2: 0.0, z1: 0.0, z2: 0.0 };
        eq.recalculate();
        eq
    }
    fn set_sample_rate(&mut self, sample_rate: f32) { self.sample_rate = sample_rate; self.recalculate(); }
    fn set_freq(&mut self, frequency: f32) { self.frequency = frequency; self.recalculate(); }
    fn set_gain(&mut self, gain: f32) { self.gain = gain; self.recalculate(); }
    fn recalculate(&mut self) {
        let angle = self.frequency as f64 * 2.0 * core::f64::consts::PI * (1.0 / self.sample_rate) as f64;
        let alpha = (libm::sin(angle) / (2.0 * REVERB_EQ_Q_DEFAULT) as f64) as f32;
        let a = libm::pow(10.0, (self.gain / 40.0) as f64) as f32;
        let a0 = (1.0_f64 + (alpha / a) as f64) as f32;
        self.b0 = (1.0_f64 + (alpha * a) as f64) / a0 as f64;
        self.a1 = -2.0 * libm::cos(angle) / a0 as f64;
        self.b2 = ((1.0 - alpha * a) / a0) as f64;
        self.a2 = ((1.0 - alpha / a) / a0) as f64;
    }
    fn process(&mut self, input: f32) -> f32 {
        let left = input as f64 - self.z2 * self.a2 - self.z1 * self.a1;
        let output = self.b2 * self.z2 + self.a1 * self.z1 + self.b0 * left;
        self.z2 = self.z1;
        self.z1 = left;
        output as f32
    }
}

/// Diffusion times in seconds for the 8 allpass diffusers
const TDIFF1: [f32; 8] = [
    20346e-6, 24421e-6, 31604e-6, 27333e-6,
    22904e-6, 29291e-6, 13458e-6, 19123e-6,
];

/// Delay times in seconds for the 8 delay lines
const TDELAY: [f32; 8] = [
    153129e-6, 210389e-6, 127837e-6, 256891e-6,
    174713e-6, 192303e-6, 125000e-6, 219991e-6,
];

/// Allpass diffuser
struct Diff1 {
    line: Vec<f32>,
    index: usize,
    size: usize,
    c: f32,
}

impl Diff1 {
    fn new(capacity: usize) -> Self {
        Self {
            line: vec![0.0; capacity],
            index: 0,
            size: 0,
            c: 0.6,
        }
    }

    fn init(&mut self, size: usize, c: f32) {
        self.size = size;
        self.c = c;
        self.line.fill(0.0);
        self.index = 0;
    }

    fn reset(&mut self) {
        self.line.fill(0.0);
        self.index = 0;
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let z = self.line[self.index];
        let x_minus_cz = x - self.c * z;
        self.line[self.index] = x_minus_cz;
        self.index += 1;
        if self.index >= self.size {
            self.index = 0;
        }
        z + self.c * x_minus_cz
    }
}

/// Tone filter for reverb decay
struct Filt1 {
    gmf: f32,
    glo: f32,
    wlo: f32,
    whi: f32,
    slo: f32,
    shi: f32,
}

impl Filt1 {
    fn new() -> Self {
        Self {
            gmf: 1.0,
            glo: 0.0,
            wlo: 0.1,
            whi: 0.1,
            slo: 0.0,
            shi: 0.0,
        }
    }

    fn reset(&mut self) {
        self.slo = 0.0;
        self.shi = 0.0;
    }

    fn set_params(&mut self, del: f32, tmf: f32, tlo: f32, wlo: f32, thi: f32, chi: f32) {
        self.gmf = libm::powf(0.001, del / tmf);
        self.glo = libm::powf(0.001, del / tlo) / self.gmf - 1.0;
        self.wlo = wlo;
        let g = libm::powf(0.001, del / thi) / self.gmf;
        let t = (1.0 - g * g) / (2.0 * g * g * chi);
        self.whi = (libm::sqrtf(1.0 + 4.0 * t) - 1.0) / (2.0 * t);
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        self.slo += self.wlo * (x - self.slo) + 1e-10;
        let x = x + self.glo * self.slo;
        self.shi += self.whi * (x - self.shi);
        self.gmf * self.shi
    }
}

/// Simple delay line for reverb
struct RevDelay {
    line: Vec<f32>,
    index: usize,
    size: usize,
}

impl RevDelay {
    fn new(capacity: usize) -> Self {
        Self {
            line: vec![0.0; capacity],
            index: 0,
            size: 0,
        }
    }

    fn init(&mut self, size: usize) {
        self.size = size;
        self.line.fill(0.0);
        self.index = 0;
    }

    fn reset(&mut self) {
        self.line.fill(0.0);
        self.index = 0;
    }

    #[inline]
    fn read(&self) -> f32 {
        self.line[self.index]
    }

    #[inline]
    fn write(&mut self, x: f32) {
        self.line[self.index] = x;
        self.index += 1;
        if self.index >= self.size {
            self.index = 0;
        }
    }
}

/// Variable delay line
struct Vdelay {
    line: Vec<f32>,
    ir: usize,
    iw: usize,
    size: usize,
}

impl Vdelay {
    fn new() -> Self {
        Self {
            line: vec![0.0; (0.1 * MAX_SAMPLE_RATE) as usize],
            ir: 0,
            iw: 0,
            size: 0,
        }
    }

    fn init(&mut self, size: usize) {
        self.size = size;
        self.line.fill(0.0);
        self.ir = 0;
        self.iw = 0;
    }

    fn reset(&mut self) {
        self.line.fill(0.0);
    }

    /// `_ir = _iw - del; if (_ir < 0) _ir += _size;` amont. Un `del` négatif (pré-retard < 20 ms, hors de
    /// la plage 20–80 ms du paramètre) place la lecture en avant de l'écriture; l'amont sortirait alors du
    /// tampon (comportement indéfini), ici on replie modulo la taille.
    fn set_delay(&mut self, del: i64) {
        let size = self.size as i64;
        let mut ir = self.iw as i64 - del;
        if ir < 0 { ir += size; }
        self.ir = ir.rem_euclid(size) as usize;
    }

    #[inline]
    fn read(&mut self) -> f32 {
        let x = self.line[self.ir];
        self.ir += 1;
        if self.ir >= self.size {
            self.ir = 0;
        }
        x
    }

    #[inline]
    fn write(&mut self, x: f32) {
        self.line[self.iw] = x;
        self.iw += 1;
        if self.iw >= self.size {
            self.iw = 0;
        }
    }
}

/// Zita Reverb
///
/// An 8-channel feedback delay network reverb based on zita-rev1.
pub struct ZitaReverb {
    /// Sample rate
    sample_rate: f32,

    /// Input variable delay lines
    vdelay: [Vdelay; 2],

    /// Allpass diffusers
    diff1: [Diff1; 8],

    /// Tone filters
    filt1: [Filt1; 8],

    /// Delay lines
    delay: [RevDelay; 8],

    /// Output EQ
    pareq: [OutputEq; 2],

    /// Dirty flags for parameter recalculation
    a_dirty: bool,
    b_dirty: bool,
    c_dirty: bool,

    /// Input delay in seconds
    ipdel: f32,

    /// Low frequency crossover
    xover: f32,

    /// Low frequency RT60
    rtlow: f32,

    /// Mid frequency RT60
    rtmid: f32,

    /// Damping frequency
    fdamp: f32,

    /// Dry/wet mix
    opmix: f32,

    /// Dry gain
    g0: f32,

    /// Wet gain
    g1: f32,
}

impl ZitaReverb {
    /// Create a new ZitaReverb
    pub fn new(sample_rate: f32) -> Self {
        let mut reverb = Self {
            sample_rate,
            vdelay: [Vdelay::new(), Vdelay::new()],
            diff1: core::array::from_fn(|i| Diff1::new((TDIFF1[i] * MAX_SAMPLE_RATE) as usize + 2)),
            filt1: core::array::from_fn(|_| Filt1::new()),
            delay: core::array::from_fn(|i| RevDelay::new(((TDELAY[i] - TDIFF1[i]) * MAX_SAMPLE_RATE) as usize + 2)),
            pareq: [OutputEq::new(sample_rate), OutputEq::new(sample_rate)],
            a_dirty: true,
            b_dirty: true,
            c_dirty: true,
            ipdel: 0.04,
            xover: 200.0,
            rtlow: 3.0,
            rtmid: 2.0,
            fdamp: 3000.0,
            opmix: 0.25,
            g0: 0.0,
            g1: 0.0,
        };


        reverb.init_delay_lines();
        reverb.prepare();

        reverb
    }

    /// Initialize delay lines based on sample rate
    fn init_delay_lines(&mut self) {
        assert!(self.sample_rate > 0.0 && self.sample_rate <= MAX_SAMPLE_RATE);
        let vdelay_size = (0.1 * self.sample_rate) as usize;
        self.vdelay[0].init(vdelay_size);
        self.vdelay[1].init(vdelay_size);

        // Initialize diffusers and delays
        for i in 0..8 {
            let k1 = (TDIFF1[i] * self.sample_rate + 0.5) as usize;
            let k2 = (TDELAY[i] * self.sample_rate + 0.5) as usize;
            let c = if i & 1 == 1 { -0.6 } else { 0.6 };
            self.diff1[i].init(k1, c);
            self.delay[i].init(k2.saturating_sub(k1).max(1));
        }
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.pareq[0].set_sample_rate(sample_rate);
        self.pareq[1].set_sample_rate(sample_rate);
        self.init_delay_lines();
        self.a_dirty = true;
        self.b_dirty = true;
        self.c_dirty = true;
        self.prepare();
    }

    /// Prepare parameters for processing
    #[allow(clippy::approx_constant)] // 6.2832f amont, volontairement différent de TAU
    fn prepare(&mut self) {
        if self.a_dirty {
            let k = libm::floorf((self.ipdel - 0.020) * self.sample_rate + 0.5) as i64;
            self.vdelay[0].set_delay(k);
            self.vdelay[1].set_delay(k);
            self.a_dirty = false;
        }

        if self.b_dirty {
            // ZitaReverb.cpp:234-238 utilise la constante float 6.2832f, pas 2π.
            let wlo = 6.2832_f32 * self.xover / self.sample_rate;
            let chi = if self.fdamp > 0.49 * self.sample_rate {
                2.0
            } else {
                1.0 - libm::cosf(6.2832_f32 * self.fdamp / self.sample_rate)
            };

            for i in 0..8 {
                self.filt1[i].set_params(
                    TDELAY[i],
                    self.rtmid,
                    self.rtlow,
                    wlo,
                    0.5 * self.rtmid,
                    chi,
                );
            }
            self.b_dirty = false;
        }

        if self.c_dirty {
            self.g0 = (1.0 - self.opmix) * (1.0 - self.opmix);
            self.g1 = 1.0 - self.g0;
            self.c_dirty = false;
        }
    }

    /// Set pre-delay in seconds
    pub fn set_delay(&mut self, v: f32) {
        self.ipdel = v;
        self.a_dirty = true;
        self.prepare();
    }

    /// Set low frequency crossover in Hz
    pub fn set_xover(&mut self, v: f32) {
        self.xover = v;
        self.b_dirty = true;
        self.prepare();
    }

    /// Set low frequency RT60 in seconds
    pub fn set_rtlow(&mut self, v: f32) {
        self.rtlow = v;
        self.c_dirty = true;
        self.prepare();
    }

    /// Set mid frequency RT60 in seconds
    pub fn set_rtmid(&mut self, v: f32) {
        self.rtmid = v;
        self.b_dirty = true;
        self.c_dirty = true;
        self.prepare();
    }

    /// Set damping frequency in Hz
    pub fn set_fdamp(&mut self, v: f32) {
        self.fdamp = v;
        self.b_dirty = true;
        self.prepare();
    }

    /// Set dry/wet mix (0.0 = dry, 1.0 = wet)
    pub fn set_mix(&mut self, v: f32) {
        self.opmix = v;
        self.c_dirty = true;
        self.prepare();
    }

    /// Set EQ frequency
    pub fn set_eq_freq(&mut self, freq: f32) {
        self.pareq[0].set_freq(freq);
        self.pareq[1].set_freq(freq);
    }

    /// Set EQ gain
    pub fn set_eq_gain(&mut self, gain: f32) {
        self.pareq[0].set_gain(gain);
        self.pareq[1].set_gain(gain);
    }

    /// Reset reverb state
    pub fn reset(&mut self) {
        self.vdelay[0].reset();
        self.vdelay[1].reset();

        for i in 0..8 {
            self.diff1[i].reset();
            self.filt1[i].reset();
            self.delay[i].reset();
        }

    }

    /// Process stereo input
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Hadamard-like mixing gain
        let g = 0.35355; // sqrt(0.125)

        // Write input to variable delay
        self.vdelay[0].write(left);
        self.vdelay[1].write(right);

        // Read from delays and process through diffusers
        let t = 0.3 * self.vdelay[0].read();
        let x0 = self.diff1[0].process(self.delay[0].read() + t);
        let x1 = self.diff1[1].process(self.delay[1].read() + t);
        let x2 = self.diff1[2].process(self.delay[2].read() - t);
        let x3 = self.diff1[3].process(self.delay[3].read() - t);

        let t = 0.3 * self.vdelay[1].read();
        let x4 = self.diff1[4].process(self.delay[4].read() + t);
        let x5 = self.diff1[5].process(self.delay[5].read() + t);
        let x6 = self.diff1[6].process(self.delay[6].read() - t);
        let x7 = self.diff1[7].process(self.delay[7].read() - t);

        // 8x8 Hadamard mixing (butterfly operations)
        let (x0, x1) = (x0 + x1, x0 - x1);
        let (x2, x3) = (x2 + x3, x2 - x3);
        let (x4, x5) = (x4 + x5, x4 - x5);
        let (x6, x7) = (x6 + x7, x6 - x7);

        let (x0, x2) = (x0 + x2, x0 - x2);
        let (x1, x3) = (x1 + x3, x1 - x3);
        let (x4, x6) = (x4 + x6, x4 - x6);
        let (x5, x7) = (x5 + x7, x5 - x7);

        let (x0, x4) = (x0 + x4, x0 - x4);
        let (x1, x5) = (x1 + x5, x1 - x5);
        let (x2, x6) = (x2 + x6, x2 - x6);
        let (x3, x7) = (x3 + x7, x3 - x7);

        // Stereo output from mix
        let mut out_l = self.g1 * (x1 + x2);
        let mut out_r = self.g1 * (x1 - x2);

        // Feed back through filters and delays
        self.delay[0].write(self.filt1[0].process(g * x0));
        self.delay[1].write(self.filt1[1].process(g * x1));
        self.delay[2].write(self.filt1[2].process(g * x2));
        self.delay[3].write(self.filt1[3].process(g * x3));
        self.delay[4].write(self.filt1[4].process(g * x4));
        self.delay[5].write(self.filt1[5].process(g * x5));
        self.delay[6].write(self.filt1[6].process(g * x6));
        self.delay[7].write(self.filt1[7].process(g * x7));

        // Apply EQ
        out_l = self.pareq[0].process(out_l);
        out_r = self.pareq[1].process(out_r);

        // Mix with dry signal
        out_l += self.g0 * left;
        out_r += self.g0 * right;

        (out_l, out_r)
    }
}

impl Default for ZitaReverb {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverb_basic() {
        let mut reverb = ZitaReverb::new(44100.0);
        reverb.set_mix(0.5);

        // Process some samples
        for i in 0..1000 {
            let input = if i < 100 { 0.5 } else { 0.0 };
            let (left, right) = reverb.process(input, input);
            assert!(left.is_finite(), "Reverb produced invalid left output");
            assert!(right.is_finite(), "Reverb produced invalid right output");
        }
    }

    #[test]
    fn test_reverb_tail() {
        let mut reverb = ZitaReverb::new(44100.0);
        reverb.set_mix(1.0); // Full wet
        reverb.set_rtmid(2.0);

        // Send impulse
        let (_, _) = reverb.process(1.0, 1.0);

        // Process more samples and check for reverb tail
        let mut max_output = 0.0f32;
        for _ in 0..44100 {
            let (left, right) = reverb.process(0.0, 0.0);
            max_output = max_output.max(left.abs()).max(right.abs());
        }

        // Should have some reverb tail
        assert!(max_output > 0.0, "Reverb should produce a tail");
    }

    #[test]
    fn test_reverb_parameters() {
        let mut reverb = ZitaReverb::new(44100.0);

        // Test all setters
        reverb.set_delay(0.05);
        reverb.set_xover(300.0);
        reverb.set_rtlow(4.0);
        reverb.set_rtmid(3.0);
        reverb.set_fdamp(5000.0);
        reverb.set_mix(0.75);
        reverb.set_eq_freq(2000.0);
        reverb.set_eq_gain(3.0);

        // Should not crash and produce valid output
        let (left, right) = reverb.process(0.5, 0.5);
        assert!(left.is_finite());
        assert!(right.is_finite());
    }

    #[test]
    fn test_reverb_reset() {
        let mut reverb = ZitaReverb::new(44100.0);
        reverb.set_mix(1.0);

        // Build up state
        for _ in 0..1000 {
            let _ = reverb.process(0.5, 0.5);
        }

        // Reset
        reverb.reset();

        // After reset, output should be minimal
        let (left, right) = reverb.process(0.0, 0.0);
        assert!(
            left.abs() < 0.001 && right.abs() < 0.001,
            "After reset, output should be near zero"
        );
    }

    #[test]
    fn test_reverb_dry_wet() {
        let input = 0.5;

        // Full dry
        let mut reverb_dry = ZitaReverb::new(44100.0);
        reverb_dry.set_mix(0.0);
        let (left, _) = reverb_dry.process(input, input);
        assert!(
            (left - input).abs() < 0.01,
            "Full dry should pass input: {} vs {}",
            left,
            input
        );

        // Full wet (no dry component)
        let mut reverb_wet = ZitaReverb::new(44100.0);
        reverb_wet.set_mix(1.0);

        // Process impulse
        let (first_l, _) = reverb_wet.process(input, input);

        // First sample of fully wet signal depends on reverb structure
        // but should not equal input (due to delay)
        assert!(first_l.is_finite());
    }
}
