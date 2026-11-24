# Billing schedule

Compose a SaaS contract billing phase with start date, duration, recurring or one-off products, invoice timing, discounts, minimum commitments, and live rolling totals.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add billing-schedule`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::billing_schedule::{BillingSchedule, BillingScheduleEvent, PhaseDuration};

let schedule = cx.new(|cx| {
    BillingSchedule::new("billing-schedule", cx)
        .with_phase("Build-out phase")
        .with_default_start("2026-01-01")
        .with_duration(PhaseDuration::TwelveMonths)
});
cx.subscribe(&schedule, |_, event: &BillingScheduleEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Edit the phase name inline in the summary bar, pick a `Start date` from the interactive calendar popover, select a `Duration` (`Milestone` through `24 months`), adjust each product's repeat cadence (`Once`, `Monthly`, `Quarterly`, `Yearly`), unit price, and timing (`billed in advance` / `billed in arrears`), or add catalog products, percentage coupon discounts, and phase minimum commitments. Press `Escape` to close any open popover.

## API

Primary GPUI type: `BillingSchedule` (`kui::components::billing_schedule::BillingSchedule`).
Event type: `BillingScheduleEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_phase(...)`, `with_currency(...)`, `with_default_start(...)`, `with_duration(...)`, `with_catalog(...)`, `with_lines(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Calendar, duration, cadence, timing, and catalog menus enter and exit smoothly via `pro_presence`, computed end dates cross-glide via `pro_view_transition`, and `First invoice`, discount savings, and `Phase total` interpolate live via `pro_rolling_number` while respecting `Theme::reduced_motion`. Below `500px`, fields stack and secondary columns collapse cleanly.

Desktop and web use the same GPUI source.
