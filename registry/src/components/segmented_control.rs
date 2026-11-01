//! SegmentedControl — selectable options using stable host keys.
use crate::primitives::form_choices::{Arrangement, ChoiceLayout, FormChoices};
pub use crate::primitives::form_choices::{FormOption, SelectionChanged};
pub struct SegmentedControlLayout;
impl ChoiceLayout for SegmentedControlLayout {
    const STYLE: Arrangement = Arrangement::Segments;
}
pub type SegmentedControl = FormChoices<SegmentedControlLayout>;
