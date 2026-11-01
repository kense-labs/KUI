//! KUI registry source.
//!
//! This crate is the authoritative tree that `kui add` copies into an
//! application's `ui/` folder. The story gallery depends on it directly so
//! preview and install source stay identical.

pub mod components;
pub mod foundation;
#[cfg(feature = "web")]
pub mod html;
pub mod primitives;

pub use foundation::{Theme, ThemeMode, Tokens, init};
pub use gpui;

pub mod prelude {
    pub use crate::foundation::{Theme, ThemeMode, Tokens, init};
    pub use gpui::prelude::*;
    pub use gpui::{
        App, Context, Div, ElementId, InteractiveElement, IntoElement, ParentElement, Render,
        SharedString, Stateful, Styled, Window, div, px, rgb,
    };
}
