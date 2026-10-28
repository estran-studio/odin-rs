//! Envelope modules

pub mod adsr;

pub use adsr::Adsr;

/// Envelope state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvelopeState {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Common envelope trait
pub trait Envelope {
    /// Process one sample, returning the envelope value
    fn process(&mut self) -> f32;

    /// Trigger the envelope (note on)
    fn trigger(&mut self);

    /// Release the envelope (note off)
    fn release(&mut self);

    /// Get current state
    fn state(&self) -> EnvelopeState;

    /// Check if envelope is active
    fn is_active(&self) -> bool;

    /// Reset envelope to idle
    fn reset(&mut self);
}
