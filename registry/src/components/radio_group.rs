//! RadioGroup — selectable options using stable host keys.
use crate::primitives::form_choices::{Arrangement, ChoiceLayout, FormChoices};
pub use crate::primitives::form_choices::{FormOption, SelectionChanged};
/// `RadioGroup` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::radio_group::{RadioGroup, FormOption};
///
/// let control = cx.new(|cx| RadioGroup::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
/// # }
/// ```
pub struct RadioGroupLayout;
impl ChoiceLayout for RadioGroupLayout {
    const STYLE: Arrangement = Arrangement::Radio;
}
pub type RadioGroup = FormChoices<RadioGroupLayout>;
