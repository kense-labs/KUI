//! Calendar on a proleptic Gregorian grid.
pub use crate::primitives::form_dates::{CivilDate, DateSpan};
use crate::primitives::form_dates::{DateControl, DateLayout};
/// `Calendar` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::calendar::{Calendar, CivilDate};
///
/// let control = cx.new(|cx| Calendar::new("Review date", CivilDate::today_utc(), cx));
/// # }
/// ```
pub struct CalendarLayout;
impl DateLayout for CalendarLayout {
    const FIELD: bool = false;
    const RANGE: bool = false;
}
pub type Calendar = DateControl<CalendarLayout>;
