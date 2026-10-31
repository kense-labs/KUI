//! An editable, filterable single-choice field.
use crate::primitives::picker::{Picker, PickerStyle};
pub struct ComboboxStyle;
impl PickerStyle for ComboboxStyle {
    const SEARCH: bool = true;
}
pub type Combobox = Picker<ComboboxStyle>;
