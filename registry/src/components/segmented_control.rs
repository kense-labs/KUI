//! SegmentedControl — selectable options using stable host keys.
use crate::primitives::form_choices::{Arrangement, ChoiceLayout, FormChoices};
pub use crate::primitives::form_choices::{FormOption, SelectionChanged};
/// `SegmentedControl` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::segmented_control::{SegmentedControl, FormOption};
///
/// let control = cx.new(|cx| SegmentedControl::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
/// # }
/// ```
pub struct SegmentedControlLayout;
impl ChoiceLayout for SegmentedControlLayout {
    const STYLE: Arrangement = Arrangement::Segments;
}
pub type SegmentedControl = FormChoices<SegmentedControlLayout>;
