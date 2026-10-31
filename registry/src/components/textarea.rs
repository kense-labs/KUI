//! A resizable, multiline text field.
use crate::primitives::{
    editor::InputMode,
    field::{Field, FieldStyle},
};

/// Defines the input behavior of this field.
pub struct TextareaStyle;
impl FieldStyle for TextareaStyle {
    const MODE: InputMode = InputMode::Multi;
}
/// A resizable, multiline text field.
pub type Textarea = Field<TextareaStyle>;
