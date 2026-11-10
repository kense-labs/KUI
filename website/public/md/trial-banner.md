# Trial banner

Show trial status as a banner, sidebar card or expandable header pill.

## Install

```bash
kui init
kui add trial-banner
```

The example uses the crate API. For copied source, replace `kui::` with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::trial_banner::{CivilDate, TrialBanner, TrialVariant};

let today = CivilDate::new(2026, 10, 27).expect("valid example date");
let ends_at = today.add_days(9);
let control = cx.new(|cx| {
    TrialBanner::new("workspace-trial", cx)
        .with_variant(TrialVariant::Banner)
        .with_schedule(today, ends_at, 9, 14)
});
```

Mount the entity as a GPUI child after `kui::init(cx)`. Feed the current local civil date, end date, whole days left and total trial length from your billing provider. Use `set_schedule(today, ends_at, days_left, trial_days, cx)` to update the same trial.

| API | Default or behavior |
| --- | --- |
| `with_variant`, `set_variant`, `is_expanded` | `TrialVariant::Banner`; also `Card` or `Pill`. `is_expanded` reports whether the pill's card is open. |
| `with_plan`, `with_workspace` | `Pro`, `Northwind`. |
| `with_schedule` | Demo uses October 27, 2026 as the reference day, 9 days remaining and a 14-day trial. Nonpositive remaining days show Expired. |
| `with_warning_days` | 3. The last positive days at or below the threshold use warning copy and tone. |
| `with_features`, `with_loss_summary` | The sample uses Okta SSO, Slack alerts, Linear issue sync and team seats. Each `TrialFeature` has a stable ID, label, optional detail and glyph. |
| `with_payment` | A host task resolving `Result<(), SharedString>`. A successful task confirms the payment method. A failure leaves the banner open with the error and a retry. Without a task, the component simulates success in 900ms and reports that it charged nothing. |
| `with_collapse` | `TrialCollapse::Pill`. Use `Hidden` when the host handles reopening. |
| `with_dismissible`, `with_snoozable` | Both default to true. The snooze action is available when there is a valid future reminder date. |
| `TrialEvent` | `ComparePlans`, `ManageBilling`, `Dismissed`, `Snoozed { choice, until }` and `Upgraded { simulated }`. |

Use the events to route to plans or billing and persist dismissals or snooze dates. The snooze date is expressed in the host's civil calendar. Only one payment task runs at a time. Dismissal or removal drops the in-flight task. Host code can use a cancellation-safe future for the request.

## Interaction and motion

Tab reaches each action. Enter and Space activate the focused control. In the snooze menu, Arrow Up and Arrow Down cycle through options, Home and End move to the edges and Escape returns to the snooze button. After snoozing or dismissal, focus moves to the remaining pill. Opening the pill focuses the primary action.

The banner wraps its actions below the copy under 600px of available width. The card fills the container up to 340px. A 240ms spring retargets the progress arc, and the heading uses a 100ms exit and 160ms entrance with a 2px rise. Reduced motion settles these values immediately.

The Desktop and Web previews use the same GPUI component.
