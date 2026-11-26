//! DatePicker on a proleptic Gregorian grid.
pub use crate::primitives::form_dates::{CivilDate, DateCleared, DateSpan};
use crate::primitives::form_dates::{DateControl, DateLayout};
/// `DatePicker` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::date_picker::{DatePicker, CivilDate};
///
/// let control = cx.new(|cx| DatePicker::new("Review date", CivilDate::today_utc(), cx));
/// # }
/// ```
pub struct DatePickerLayout;
impl DateLayout for DatePickerLayout {
    const FIELD: bool = true;
    const RANGE: bool = false;
}
pub type DatePicker = DateControl<DatePickerLayout>;
