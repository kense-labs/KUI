//! Calendar on a proleptic Gregorian grid.
pub use crate::primitives::form_dates::{CivilDate, DateSpan};
use crate::primitives::form_dates::{DateControl, DateLayout};
pub struct CalendarLayout;
impl DateLayout for CalendarLayout {
    const FIELD: bool = false;
    const RANGE: bool = false;
}
pub type Calendar = DateControl<CalendarLayout>;
