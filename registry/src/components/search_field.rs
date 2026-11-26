//! A query field with a reserved clear action.
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
/// use kui::components::search_field::SearchField;
///
/// let control = cx.new(|cx| SearchField::new("query", "Search projects", window, cx).with_placeholder("Search by name", cx));
/// # }
/// ```
pub struct SearchFieldStyle;
impl FieldStyle for SearchFieldStyle {
    const MODE: InputMode = InputMode::Search;
}
/// A query field with a reserved clear action.
pub type SearchField = Field<SearchFieldStyle>;
