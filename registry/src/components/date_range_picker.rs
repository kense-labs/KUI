//! DateRangePicker on a proleptic Gregorian grid.
pub use crate::primitives::form_dates::{CivilDate, DateSpan};
use crate::primitives::form_dates::{DateControl, DateLayout};
/// `DateRangePicker` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::date_range_picker::{DateRangePicker, CivilDate};
///
/// let control = cx.new(|cx| DateRangePicker::new("Review date", CivilDate::today_utc(), cx));
/// # }
/// ```
pub struct DateRangePickerLayout;
impl DateLayout for DateRangePickerLayout {
    const FIELD: bool = true;
    const RANGE: bool = true;
}
pub type DateRangePicker = DateControl<DateRangePickerLayout>;
