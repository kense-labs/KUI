//! KUI registry source.
//!
//! This crate is the authoritative tree that `kui add` copies into an
//! application's `ui/` folder. The story gallery depends on it directly so
//! preview and install source stay identical.

pub mod components;
pub mod foundation;
pub mod primitives;

pub use foundation::{init, Theme, ThemeMode, Tokens};
pub use gpui;

pub mod prelude {
    pub use crate::foundation::{init, Theme, ThemeMode, Tokens};
    pub use gpui::prelude::*;
    pub use gpui::{
        div, px, rgb, App, Context, Div, ElementId, InteractiveElement, IntoElement,
        ParentElement, Render, SharedString, Stateful, Styled, Window,
    };
}
