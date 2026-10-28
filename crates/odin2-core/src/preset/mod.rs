//! Odin 2 Preset Parser
//!
//! Parses .odin preset files (JUCE ValueTree binary format)

#[cfg(feature = "std")]
mod parser;
#[cfg(feature = "std")]
mod value_tree;
#[cfg(feature = "std")]
mod interpolation;

#[cfg(feature = "std")]
pub use parser::{OdinPreset, PresetAudit, PresetUiState};
#[cfg(feature = "std")]
pub use value_tree::{ValueTree, ValueTreeNode, ValueTreeError, VariantValue};

// Re-export types that work in no_std
mod types;
pub use types::*;
