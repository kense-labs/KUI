//! MultiSelect — selectable options using stable host keys.
use crate::primitives::form_choices::{Arrangement, ChoiceLayout, FormChoices};
pub use crate::primitives::form_choices::{FormOption, SelectionChanged};
/// `MultiSelect` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::multi_select::{MultiSelect, FormOption};
///
/// let control = cx.new(|cx| MultiSelect::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
/// # }
/// ```
pub struct MultiSelectLayout;
impl ChoiceLayout for MultiSelectLayout {
    const STYLE: Arrangement = Arrangement::Multi;
}
pub type MultiSelect = FormChoices<MultiSelectLayout>;
