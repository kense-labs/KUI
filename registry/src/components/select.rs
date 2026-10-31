//! A single-choice field with directional value transitions.
use crate::primitives::picker::{Picker, PickerStyle};
pub struct SelectStyle;
impl PickerStyle for SelectStyle {
    const SEARCH: bool = false;
}
pub type Select = Picker<SelectStyle>;
