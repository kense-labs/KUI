//! KUI shared design tokens, motion presets, and theme globals.
//!
//! Install once per application (`kui add kui-foundation`). Components never
//! reinstall this module.

mod init;
mod motion;
mod theme;
mod tokens;

pub use init::init;
pub use motion::{MOTION, MotionPreset, MotionTokens};
pub use theme::{AccentPalette, Theme, ThemeMode};
pub use tokens::{ColorTokens, RadiusTokens, SURFACE_RADIUS, SpaceTokens, Tokens};

/// Radius shared by control chrome, panels, highlights, and focus outlines.
pub fn surface_radius(cx: &gpui::App) -> gpui::Pixels {
    cx.try_global::<Theme>()
        .map_or(SURFACE_RADIUS, |theme| theme.tokens.radius.md)
}
