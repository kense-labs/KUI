//! A labelled, single-line text field.
use crate::primitives::{
    editor::InputMode,
    field::{Field, FieldStyle},
};

/// Defines the input behavior of this field.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::input::Input;
///
/// let control = cx.new(|cx| Input::new("project", "Project name", window, cx).with_description("A short descriptive name."));
/// # }
/// ```
pub struct InputStyle;
impl FieldStyle for InputStyle {
    const MODE: InputMode = InputMode::Single;
}
/// A labelled, single-line text field.
pub type Input = Field<InputStyle>;
