//! KUI shared design tokens, motion presets, and theme globals.
//!
//! Install once per application (`kui add kui-foundation`). Components never
//! reinstall this module.

mod init;
mod motion;
mod theme;
mod tokens;

pub use init::init;
pub use motion::{MotionPreset, MotionTokens, MOTION};
pub use theme::{Theme, ThemeMode};
pub use tokens::{ColorTokens, RadiusTokens, SpaceTokens, Tokens};
