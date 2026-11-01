//! KUI components for GPUI on native desktop and WASM web.
//!
//! Install the `kense-kui` package with the dependency name `kui`:
//! `cargo add kense-kui --rename kui --no-default-features`.
//! Call [`init`] before opening windows, then use controls from [`components`].
//! Disable default features for WASM; `test-support` is for host smoke tests.
//! The `kui` CLI copies this same source into an application's `ui/` folder.
//!
//! # Examples
//!
//! ```
//! use kui::components::button::Button;
//!
//! let button = Button::new("save").primary().label("Save");
//! ```

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
