//! Tables dessinées partagées, construites hors temps réel (WavetableContainer.cpp:185–452).
//! Amont les génère toujours avec samplerate 44100 (PluginProcessorConstructor.cpp:188–190,
//! PluginProcessorReadPatch.cpp:185–199, OscComponent.cpp:1279–1302).
use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::constants::{SUBTABLES_PER_WAVETABLE, WAVETABLE_LENGTH};
use crate::preset::OscillatorParams;

const BUILD_SAMPLE_RATE: f32 = 44100.0;
const NUMBER_OF_HARMONICS: usize = 256;
type Table = [[f32; WAVETABLE_LENGTH]; SUBTABLES_PER_WAVETABLE];

/// Neuf tables: WaveDraw 1–3, ChipDraw 1–3, SpecDraw 1–3.
pub struct DrawTableBank { tables: Vec<Box<Table>> }

impl DrawTableBank {
    /// Alloue et calcule; à appeler uniquement au chargement du patch.
    pub fn new(params: &[OscillatorParams; 3]) -> Self {
        let mut sine_lookup = [0.0f32; WAVETABLE_LENGTH];
        for (i, value) in sine_lookup.iter_mut().enumerate() {
            *value = libm::sin(core::f64::consts::TAU * i as f64 / WAVETABLE_LENGTH as f64) as f32;
        }
        let mut tables = Vec::with_capacity(9);
        for kind in 0..3 {
            for osc in params {
                let mut sine = [0.0f32; NUMBER_OF_HARMONICS];
                let mut cosine = [0.0f32; NUMBER_OF_HARMONICS];
                let limit_cap = if kind == 2 { 50 } else { NUMBER_OF_HARMONICS };
                if kind == 2 {
                    for h in 1..50 { sine[h] = osc.specdraw[h - 1] / libm::sqrtf(h as f32); }
                } else {
                    let values: &[f32] = if kind == 0 { &osc.wavedraw } else { &osc.chipdraw };
                    segment_coefficients(values, kind == 1, &mut sine, &mut cosine);
                }
                let mut table = Box::new([[0.0f32; WAVETABLE_LENGTH]; SUBTABLES_PER_WAVETABLE]);
                let mut seed = 27.5f64;
                let mut maximum = 0.0f32;
                for sub in table.iter_mut() {
                    let limit = ((BUILD_SAMPLE_RATE as f64 * 0.5 / seed - 1.0) as i64).clamp(0, limit_cap as i64) as usize;
                    for (position, value) in sub.iter_mut().enumerate() {
                        let mut sum = 0.0f32;
                        for h in 1..limit {
                            let index = (position * h) % WAVETABLE_LENGTH;
                            sum += sine[h] * sine_lookup[index]
                                + cosine[h] * sine_lookup[(index + WAVETABLE_LENGTH / 4) % WAVETABLE_LENGTH];
                        }
                        *value = sum;
                        maximum = maximum.max(sum.abs());
                    }
                    seed *= 1.1892071150;
                }
                let scale = if maximum > 1e-5 { 1.0 / maximum } else { maximum };
                for sub in table.iter_mut() { for value in sub.iter_mut() { *value *= scale; } }
                tables.push(table);
            }
        }
        Self { tables }
    }
    /// `draw` 0..8, dans l'ordre WaveDraw 1–3, ChipDraw 1–3, SpecDraw 1–3.
    pub fn subtable(&self, draw: usize, sub: usize) -> &[f32; WAVETABLE_LENGTH] {
        &self.tables[draw.min(8)][sub.min(SUBTABLES_PER_WAVETABLE - 1)]
    }
}

/// const_segment_* (chipdraw) et lin_segment_* (wavedraw), WavetableContainer.cpp:412–453.
fn segment_coefficients(values: &[f32], constant: bool, sine: &mut [f32], cosine: &mut [f32]) {
    let width = (core::f64::consts::TAU / values.len() as f64) as f32 as f64;
    for h in 1..NUMBER_OF_HARMONICS {
        let n = h as f64;
        let (mut s, mut c) = (0.0f64, 0.0f64);
        for (segment, &fa) in values.iter().enumerate() {
            let a = segment as f64 * width;
            let b = (segment + 1) as f64 * width;
            let (sa, sb, ca, cb) = (libm::sin(a * n), libm::sin(b * n), libm::cos(a * n), libm::cos(b * n));
            if constant {
                s += fa as f64 / n * (ca - cb);
                c += fa as f64 / n * (sb - sa);
            } else {
                let fb = values[(segment + 1) % values.len()] as f64;
                let slope = (fb - fa as f64) / (b - a);
                let offset = fa as f64 - slope * a;
                s += (slope * (sb - sa) + n * (a * slope + offset) * ca - n * (b * slope + offset) * cb) / n / n;
                c += (-n * (a * slope + offset) * sa - slope * ca + n * (b * slope + offset) * sb + slope * cb) / n / n;
            }
        }
        sine[h] = s as f32;
        cosine[h] = c as f32;
    }
}
