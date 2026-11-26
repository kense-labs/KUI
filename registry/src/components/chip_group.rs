//! ChipGroup — selectable options using stable host keys.
use crate::primitives::form_choices::{Arrangement, ChoiceLayout, FormChoices};
pub use crate::primitives::form_choices::{FormOption, SelectionChanged};
/// `ChipGroup` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::chip_group::{ChipGroup, FormOption};
///
/// let control = cx.new(|cx| ChipGroup::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
/// # }
/// ```
pub struct ChipGroupLayout;
impl ChoiceLayout for ChipGroupLayout {
    const STYLE: Arrangement = Arrangement::Chips;
}
pub type ChipGroup = FormChoices<ChipGroupLayout>;
