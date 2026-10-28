//! Effects modules

pub mod bitcrusher;
pub mod chain;
mod common;
pub mod chorus;
pub mod delay;
pub mod distortion;
pub mod flanger;
pub mod parametric_eq;
pub mod phaser;
pub mod reverb;
pub mod ring_mod;

pub use bitcrusher::Bitcrusher;
pub use chain::EffectsChain;
pub use chorus::Chorus;
pub use delay::Delay;
pub use distortion::OversamplingDistortion;
pub use flanger::Flanger;
pub use parametric_eq::{MultibandEQ, ParametricEQ};
pub use phaser::Phaser;
pub use reverb::ZitaReverb;
pub use ring_mod::RingModulator;
