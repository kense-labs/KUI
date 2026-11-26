//! An editable, filterable single-choice field.
use crate::primitives::picker::{Picker, PickerStyle};
/// `Combobox` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::combobox::Combobox;
/// use kui::primitives::picker::PickerOption;
///
/// let control = cx.new(|cx| Combobox::new("model", "Model", vec![
///         PickerOption::new("fast", "Fast").with_keywords(["quick"]),
///         PickerOption::new("reasoning", "Reasoning"),
///     ], window, cx).with_value("fast"));
/// # }
/// ```
pub struct ComboboxStyle;
impl PickerStyle for ComboboxStyle {
    const SEARCH: bool = true;
}
pub type Combobox = Picker<ComboboxStyle>;
