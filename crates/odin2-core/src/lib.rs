//! Odin 2 Synthesizer Core DSP Engine
//!
//! A Rust no_std port of the Odin 2 synthesizer's DSP core.
//! Based on the original C++ implementation by TheWaveWarden.
//!
//! # Features
//! - `std` - Enable standard library support (for testing)
//! - `harmonium` - Enable integration with Harmonium audio engine

#![no_std]
#![allow(clippy::excessive_precision)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod dsp;
pub mod engine;
pub mod voice;
pub mod mod_matrix;
pub mod wavetables;
pub mod preset;
pub mod config;
pub mod parameters;
mod voice_manager;
mod arpeggiator;

// Re-exports
pub use engine::{OdinEngine, SynthEngine};
pub use voice::Voice;
pub use mod_matrix::ModMatrix;
pub use config::EngineConfig;
pub use parameters::{ParameterDescriptor, ParameterError, ParameterId, ParameterKind};

/// Odin 2 constants
pub mod constants {
    /// Wavetable length in samples
    pub const WAVETABLE_LENGTH: usize = 512;

    /// Number of subtables per wavetable (for different frequency ranges)
    pub const SUBTABLES_PER_WAVETABLE: usize = 33;

    /// Total number of 1D wavetables
    pub const NUMBER_OF_WAVETABLES: usize = 160;

    /// Maximum number of harmonics
    pub const NUMBER_OF_HARMONICS: usize = 256;

    /// Number of voices for polyphony
    pub const VOICES: usize = 24;

    /// Number of oscillators per voice
    pub const OSCILLATORS_PER_VOICE: usize = 3;

    /// Number of filters per voice
    pub const FILTERS_PER_VOICE: usize = 2;

    /// Number of LFOs per voice
    pub const LFOS_PER_VOICE: usize = 3;

    /// Number of envelopes per voice
    pub const ENVELOPES_PER_VOICE: usize = 3;

    /// Number of modulation matrix rows
    pub const MOD_MATRIX_ROWS: usize = 9;

    /// Modulation destinations per row
    pub const MOD_DESTINATIONS_PER_ROW: usize = 2;

    /// Minimum oscillator frequency (Hz)
    pub const OSC_FREQ_MIN: f32 = 20.0;

    /// Maximum oscillator frequency (Hz)
    pub const OSC_FREQ_MAX: f32 = 20480.0;

    /// Default oscillator frequency (A4)
    pub const OSC_FREQ_DEFAULT: f32 = 440.0;


    /// Pi constant
    pub const PI: f32 = core::f32::consts::PI;

    /// Two pi constant
    pub const TWO_PI: f32 = 2.0 * PI;
}
