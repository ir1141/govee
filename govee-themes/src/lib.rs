//! Theme engine for Govee LED strip animations.
//!
//! Provides the [`Behavior`] enum (13 animation types), built-in theme definitions,
//! palette interpolation, and user-theme loading from TOML files.

/// Core types: theme definitions, behavior enum, color utilities.
pub mod themes;
/// Built-in theme definitions across 5 categories.
pub mod theme_defs;
/// Theme loader that merges user TOML themes with builtins.
pub mod theme_loader;

pub use themes::{ThemeDef, ThemeKind, Behavior, Delay, PA, WaveParam, Rgb};
pub use theme_defs::{builtin_themes, BUILTIN_CATEGORIES};
pub use theme_loader::load_all_themes;
