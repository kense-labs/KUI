//! A masked text field with a reversible reveal control.
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
/// use kui::components::password_field::PasswordField;
///
/// let control = cx.new(|cx| PasswordField::new("access", "Access key", window, cx));
/// # }
/// ```
pub struct PasswordFieldStyle;
impl FieldStyle for PasswordFieldStyle {
    const MODE: InputMode = InputMode::Password;
}
/// A masked text field with a reversible reveal control.
pub type PasswordField = Field<PasswordFieldStyle>;
