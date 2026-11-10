# Booking pill

Choose a party size, date and time in one bottom-anchored control. The host supplies availability and receives a booking only after review.

## Installation

```sh
kui add booking-pill
```

## Usage

```rust
use chrono::NaiveDate;
use gpui::{Task, prelude::*};
use kui::components::booking_pill::{BookingPill, BookingStepChanged};

let today = NaiveDate::from_ymd_opt(2026, 9, 21).expect("valid sample date");
let control = cx.new(|cx| {
    BookingPill::new("Olea", today, cx)
        .with_venue_detail("432 Octavia Street")
        .with_confirm(|booking, _| {
            let _selected_table = booking;
            Task::ready(Ok(()))
        })
});
window.subscribe(&control, cx, |_, _: &BookingStepChanged, _, _| {}).detach();
```

## API

| Builder or method | Default | Behavior |
| --- | --- | --- |
| `new(venue, NaiveDate, &mut Context<Self>)` | Required venue and first day | Begin on the first available day. The date is supplied by the host. |
| `with_venue_detail`, `with_label`, `with_id`, `with_style` | Empty detail, “Book a table”, `booking-pill`, no override | Set ticket detail, trigger text, stable identity and root style. |
| `with_days(usize)` | 21 | Show 1 to 366 days, capped at the civil date range. |
| `with_times(Vec<NaiveTime>)` | 17:00 to 22:30 every 30 minutes | Keep the supplied order and one entry per minute. |
| `with_availability(Fn(NaiveDate, NaiveTime, u16) -> bool)` | All slots free | Evaluate available dates and slots for the current party. |
| `with_preferred_time(NaiveTime)` | 19:30 | Prefer that time, or the nearest available time on the selected day. |
| `with_party_range(u16, u16)`, `with_default_party(u16)` | 1–12, initial 2 | Constrain the party counter and its default. |
| `with_hour_cycle(BookingHourCycle)` | `Twelve` | `TwentyFour` changes display only. Booking times stay typed and local. |
| `with_confirm(Fn(Booking, &mut App) -> Task<Result<(), SharedString>>)` | Local simulated success | Resolve to show Booked. A failure keeps the ticket and allows a retry. |
| `refresh_availability(&mut Context<Self>)` | — | Refresh the current party's cached availability. The reviewed slot remains fixed and is checked again on submit. |
| `step()`, `selection()`, `BookingStepChanged(pub BookingStep)` | `Start` | Inspect the visible step or available selection and subscribe to face changes. |

## Interaction

Arrow keys move by one day or among available time slots. Page Up and Page Down move by a week in the date strip. Home and End select the first or last entry. Enter continues from an available date. Escape goes back one step. While a booking is pending, repeat confirmation and backward navigation are held until the host responds.

The pill grows from a shared bottom edge. Its width and height spring to the current face, the date strip follows horizontal drags, and a fast release glides to a day. Reduced motion settles the shape and date directly. Place the control at the bottom of a host container to keep the edge anchored.
