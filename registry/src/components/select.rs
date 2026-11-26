//! A single-choice field with directional value transitions.
use crate::primitives::picker::{Picker, PickerStyle};
/// `Select` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::select::Select;
/// use kui::primitives::picker::PickerOption;
///
/// let control = cx.new(|cx| Select::new("team", "Team", vec![
///         PickerOption::new("design", "Design"), PickerOption::new("engineering", "Engineering"),
///     ], window, cx).with_value("design"));
/// # }
/// ```
pub struct SelectStyle;
impl PickerStyle for SelectStyle {
    const SEARCH: bool = false;
}
pub type Select = Picker<SelectStyle>;
