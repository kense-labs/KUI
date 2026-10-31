//! A labelled, single-line text field.
use crate::primitives::{
    editor::InputMode,
    field::{Field, FieldStyle},
};

/// Defines the input behavior of this field.
pub struct InputStyle;
impl FieldStyle for InputStyle {
    const MODE: InputMode = InputMode::Single;
}
/// A labelled, single-line text field.
pub type Input = Field<InputStyle>;
