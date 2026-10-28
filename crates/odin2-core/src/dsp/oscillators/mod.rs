//! Oscillator modules

pub mod analog;
pub mod chiptune;
pub mod drift;
pub mod fm;
pub mod lfo;
pub mod multi;
pub mod noise;
pub mod pm;
pub mod vector;
pub mod wavetable;
pub mod wavetable2d;
pub mod draw;
pub mod preset_oscillator;
pub use draw::DrawTableBank;
pub use preset_oscillator::{OscillatorModulation, PresetOscillator};

pub use analog::AnalogOscillator;
pub use chiptune::ChiptuneOscillator;
pub use drift::DriftGenerator;
pub use fm::{FmMode, FmOscillator};
pub use lfo::{Lfo, LfoWaveform};
pub use multi::MultiOscillator;
pub use noise::NoiseOscillator;
pub use pm::PmOscillator;
pub use vector::VectorOscillator;
pub use wavetable::WavetableOscillator;
pub use wavetable2d::{WavetableOsc2D, WAVETABLE_2D_PRESETS, NUMBER_OF_WAVETABLES_2D};

/// Waveform types for analog oscillator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Waveform {
    #[default]
    Saw = 0,
    Square = 1,
    Triangle = 2,
    Sine = 3,
}

/// Common oscillator trait
pub trait Oscillator {
    /// Process one sample
    fn process(&mut self) -> f32;

    /// Set the oscillator frequency in Hz
    fn set_frequency(&mut self, freq: f32);

    /// Set the sample rate
    fn set_sample_rate(&mut self, sample_rate: f32);

    /// Reset oscillator state
    fn reset(&mut self);

    /// Randomize phase (for unison detuning)
    fn randomize_phase(&mut self);
}
