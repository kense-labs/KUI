//! RadioGroup — selectable options using stable host keys.
use crate::primitives::form_choices::{Arrangement, ChoiceLayout, FormChoices};
pub use crate::primitives::form_choices::{FormOption, SelectionChanged};
pub struct RadioGroupLayout;
impl ChoiceLayout for RadioGroupLayout {
    const STYLE: Arrangement = Arrangement::Radio;
}
pub type RadioGroup = FormChoices<RadioGroupLayout>;
