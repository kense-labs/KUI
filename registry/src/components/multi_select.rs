//! MultiSelect — selectable options using stable host keys.
use crate::primitives::form_choices::{Arrangement, ChoiceLayout, FormChoices};
pub use crate::primitives::form_choices::{FormOption, SelectionChanged};
pub struct MultiSelectLayout;
impl ChoiceLayout for MultiSelectLayout {
    const STYLE: Arrangement = Arrangement::Multi;
}
pub type MultiSelect = FormChoices<MultiSelectLayout>;
