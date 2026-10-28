//! Accès hôte générique au catalogue audio d'Odin 2, sans allocation.
//! Résoudre le nom une fois, puis conserver `ParameterId` dans le callback audio.

use crate::config::EngineConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterKind { Continuous, Integer, Boolean }

#[derive(Clone, Copy, Debug)]
pub struct ParameterDescriptor {
    pub name: &'static str,
    pub minimum: f32,
    pub maximum: f32,
    pub default: f32,
    pub kind: ParameterKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterError { UnknownParameter, NonFinite }

impl core::fmt::Display for ParameterError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::UnknownParameter => "paramètre Odin 2 inconnu",
            Self::NonFinite => "valeur de paramètre non finie",
        })
    }
}
impl core::error::Error for ParameterError {}

/// Index validé dans le catalogue de cette version de la bibliothèque.
/// Sérialiser le nom, pas cet index (le catalogue peut évoluer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParameterId(usize);

struct Binding {
    descriptor: ParameterDescriptor,
    set: fn(&mut EngineConfig, f32),
    get: fn(&EngineConfig) -> f32,
}

macro_rules! kind {
    (float) => { ParameterKind::Continuous };
    (int) => { ParameterKind::Integer };
    (bool) => { ParameterKind::Boolean };
}
macro_rules! store {
    ($target:expr, $value:expr, float) => { $target = $value };
    ($target:expr, $value:expr, int) => { $target = libm::roundf($value) as i32 };
    ($target:expr, $value:expr, bool) => { $target = $value >= 0.5 };
}
macro_rules! load {
    ($value:expr, float) => { $value };
    ($value:expr, int) => { $value as f32 };
    ($value:expr, bool) => { if $value { 1.0 } else { 0.0 } };
}
macro_rules! bindings {
    (; $(($id:literal, $root:ident $(.$field:ident)? $([$index:literal].$sub:ident)?, $kind:ident, $min:expr, $max:expr, $default:expr)),* $(,)?) => {
        static BINDINGS: &[Binding] = &[$(Binding {
            descriptor: ParameterDescriptor { name: $id, minimum: $min, maximum: $max, default: $default, kind: kind!($kind) },
            set: |config, value| { store!(config.$root $(.$field)? $([$index].$sub)?, value, $kind); },
            get: |config| load!(config.$root $(.$field)? $([$index].$sub)?, $kind),
        }),*];
    };
}
crate::odin_audio_parameters!(bindings);

impl ParameterId {
    pub fn from_name(name: &str) -> Result<Self, ParameterError> {
        BINDINGS.iter().position(|binding| binding.descriptor.name == name)
            .map(Self).ok_or(ParameterError::UnknownParameter)
    }
    pub fn from_index(index: usize) -> Option<Self> {
        if index < BINDINGS.len() { Some(Self(index)) } else { None }
    }
    pub fn descriptor(self) -> &'static ParameterDescriptor { &BINDINGS[self.0].descriptor }
    pub fn all() -> impl ExactSizeIterator<Item = Self> { (0..BINDINGS.len()).map(Self) }
    pub fn get(self, config: &EngineConfig) -> f32 { (BINDINGS[self.0].get)(config) }
    /// Valeur physique, bornée comme le paramètre JUCE correspondant.
    pub fn set(self, config: &mut EngineConfig, value: f32) -> Result<(), ParameterError> {
        if !value.is_finite() { return Err(ParameterError::NonFinite); }
        let binding = &BINDINGS[self.0];
        (binding.set)(config, value.clamp(binding.descriptor.minimum, binding.descriptor.maximum));
        Ok(())
    }
}

pub(crate) fn apply_defaults(config: &mut EngineConfig) {
    for binding in BINDINGS { (binding.set)(config, binding.descriptor.default); }
}
