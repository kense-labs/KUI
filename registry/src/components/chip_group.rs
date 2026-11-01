//! ChipGroup — selectable options using stable host keys.
use crate::primitives::form_choices::{Arrangement, ChoiceLayout, FormChoices};
pub use crate::primitives::form_choices::{FormOption, SelectionChanged};
pub struct ChipGroupLayout;
impl ChoiceLayout for ChipGroupLayout {
    const STYLE: Arrangement = Arrangement::Chips;
}
pub type ChipGroup = FormChoices<ChipGroupLayout>;
