//! A resizable, multiline text field.
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
/// use kui::components::textarea::Textarea;
///
/// let control = cx.new(|cx| Textarea::new("notes", "Project note", window, cx).with_value("Ready for review.", window, cx));
/// # }
/// ```
pub struct TextareaStyle;
impl FieldStyle for TextareaStyle {
    const MODE: InputMode = InputMode::Multi;
}
/// A resizable, multiline text field.
pub type Textarea = Field<TextareaStyle>;
