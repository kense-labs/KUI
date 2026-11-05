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
pub mod direct_touch;
pub mod family;
pub mod glyph;
pub(crate) mod kense_mark;
pub mod menu;
pub mod phone_flags;
pub mod strip;

pub mod disclosure;
pub mod editor;
pub(crate) mod emoji_catalog;
pub mod field;
pub mod flyout;
pub mod form_choices;
pub mod form_chrome;
pub mod form_dates;
pub mod form_files;
pub mod ops_surface;
pub mod picker;
pub mod plot;
pub mod presentation;

pub mod nav;

pub mod color_motion;
