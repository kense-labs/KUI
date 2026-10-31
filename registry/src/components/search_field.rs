//! A query field with a reserved clear action.
use crate::primitives::{
    editor::InputMode,
    field::{Field, FieldStyle},
};

/// Defines the input behavior of this field.
pub struct SearchFieldStyle;
impl FieldStyle for SearchFieldStyle {
    const MODE: InputMode = InputMode::Search;
}
/// A query field with a reserved clear action.
pub type SearchField = Field<SearchFieldStyle>;
