//! DateRangePicker on a proleptic Gregorian grid.
pub use crate::primitives::form_dates::{CivilDate, DateSpan};
use crate::primitives::form_dates::{DateControl, DateLayout};
pub struct DateRangePickerLayout;
impl DateLayout for DateRangePickerLayout {
    const FIELD: bool = true;
    const RANGE: bool = true;
}
pub type DateRangePicker = DateControl<DateRangePickerLayout>;
