//! KUI unstyled behavior roots.
//!
//! Presentation belongs to `components`. These modules own focus, activation,
//! and accessibility seams.

pub mod button;
pub mod sizing;

pub use button::ButtonBehavior;
pub use sizing::Size;
pub mod clipboard;
pub mod commit;
pub mod control;
pub mod family;
pub mod glyph;
pub mod strip;
