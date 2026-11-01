//! DatePicker on a proleptic Gregorian grid.
pub use crate::primitives::form_dates::{CivilDate, DateCleared, DateSpan};
use crate::primitives::form_dates::{DateControl, DateLayout};
pub struct DatePickerLayout;
impl DateLayout for DatePickerLayout {
    const FIELD: bool = true;
    const RANGE: bool = false;
}
pub type DatePicker = DateControl<DatePickerLayout>;
