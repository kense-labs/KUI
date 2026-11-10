# Date reel

Pick a civil date or time by turning cylindrical reels. Use the date and time layout for scheduling, the date layout for calendar dates, and the time layout for clock values.

## Installation

```sh
kui add date-reel
```

## Usage

```rust
use chrono::NaiveDate;
use gpui::{Task, prelude::*};
use kui::components::date_reel::{DateReel, DateReelChanged, MinuteStep};

let today = NaiveDate::from_ymd_opt(2026, 9, 24).expect("valid sample date");
let start = today.succ_opt()
    .and_then(|day| day.and_hms_opt(9, 0, 0))
    .expect("valid sample time");

let control = cx.new(|cx| {
    DateReel::new(cx)
        .with_today(today)
        .with_default_value(start)
        .with_title("Send later")
        .with_minute_step(MinuteStep::Five)
        .with_confirm(|value, _| {
            let _scheduled_civil_time = value;
            Task::ready(Ok(()))
        })
});
window.subscribe(&control, cx, |_, _: &DateReelChanged, _, _| {}).detach();
```

## API

| Builder or method | Default | Behavior |
| --- | --- | --- |
| `with_mode(DateReelMode)` | `DateTime` | Day, hour, minute and period. `Date` shows month, day-of-month and year. `Time` shows time reels. |
| `with_default_value(NaiveDateTime)` | 2026-09-24 09:30 | Uncontrolled starting value in the host's civil time. `Date` preserves the time portion, and the time layouts round to the minute grid. |
| `with_value(NaiveDateTime)`, `set_value(NaiveDateTime, &mut Context<Self>)`, `value()` | Uncontrolled | The controlled builder requests host acceptance. After `DateReelChanged`, write back the accepted value with `set_value`. If the host keeps its previous value, the reel returns there. Programmatic changes use the closest equivalent item in looping columns and emit no event. |
| `with_today(NaiveDate)`, `set_today(NaiveDate, &mut Context<Self>)` | Day of the initial value | Sets the reference for relative dates, defaults and preset functions. The committed date is preserved when it remains in bounds. |
| `with_min_date(NaiveDate)`, `with_max_date(NaiveDate)` | −30 days / +1 calendar year, or −100 / +10 calendar years in `Date` | Inclusive calendar bounds within years 1–9999. Reversed bounds are sorted. |
| `with_minute_step(MinuteStep)` | `Five` | One, five, ten, fifteen or thirty minutes. |
| `with_hour_cycle(HourCycle)` | `Twelve` | Use an AM/PM reel or show 24 hours and omit the period. |
| `with_locale(DateReelLocale)`, `with_labels(DateReelLabels)`, `with_formatter(Fn)` | English | Choose built-in English or Chinese, override labels, or format entries and summaries in a host locale. A formatter can return `None` for the built-in fallback. |
| `with_presets(Vec<DateReelPreset>)` | This evening, Tomorrow and Monday in `DateTime` | Accepts fixed civil times and functions of today. An empty vector hides quick picks. |
| `with_title(impl Into<SharedString>)`, `with_label(impl Into<SharedString>)` | No title, “Date and time” | Optional display heading and accessible group name. |
| `with_confirm(Fn(NaiveDateTime, &mut App) -> Task<Result<(), SharedString>>)` | Hidden | Confirm only a settled value. Pending blocks duplicate submissions. Failure preserves the value and permits retry. Resolve an immediate operation with `Task::ready`. |
| `with_intro(bool)`, `with_speed(f32)`, `with_paused(bool)`, `set_paused(bool, &mut Context<Self>)` | `true`, `1`, `false` | Staggered intro, positive finite speed and optional immediate landing. Reduced motion skips automatic travel. |
| `with_accent(Hsla)`, `with_id(impl Into<ElementId>)`, `with_style(Fn(Div) -> Div)`, `with_visibility(Fn(&App) -> bool)` | Theme accent, `date-reel`, no overrides | Host controls accent, identity, card chrome and optional visibility. |

`DateReelChanged(pub NaiveDateTime)` fires once when a changed user value settles. The host can convert the civil time to an instant using its own timezone rules.

## Interaction and motion

- Drag or flick vertically, scroll over a reel, or tap a visible row. A release keeps velocity, slows into a half-row handoff and snaps. Finite reels resist overscroll. Quick picks stagger the affected reels.
- Tab moves through reels and controls. Left and Right change columns. Up and Down turn a row, Page Up and Page Down turn a larger step, and Home and End select the endpoints. Type an hour, minute, year, month prefix or period to find a matching entry.
- In date mode, invalid days are dimmed and the day reel corrects itself before notifying the host. Reduced motion and paused mode land immediately, with direct drag still available.
- The lens and row height are 42px at wide sizes and 36px below 480px. The confirm action moves below quick picks and is 44px high at narrow sizes. Idle and hidden reels stop requesting frames.
