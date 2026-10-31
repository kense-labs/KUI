//! A masked text field with a reversible reveal control.
use crate::primitives::{
    editor::InputMode,
    field::{Field, FieldStyle},
};

/// Defines the input behavior of this field.
pub struct PasswordFieldStyle;
impl FieldStyle for PasswordFieldStyle {
    const MODE: InputMode = InputMode::Password;
}
/// A masked text field with a reversible reveal control.
pub type PasswordField = Field<PasswordFieldStyle>;
