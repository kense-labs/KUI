# Activity rings

Compare daily goals on concentric progress rings.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add activity-rings`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::activity_rings::{ActivityDay, ActivityGoal, ActivityRings};

let goals = vec![ActivityGoal::new("move", "Move", 500.0, "kcal", gpui::rgb(0xe96080).into())];
let days = vec![ActivityDay::new("today", "Today", vec![("move".into(), 420.0)])];
let control = cx.new(|cx| ActivityRings::new("Daily activity", goals, days, cx));
```

## Interaction

Supply stable goal and day IDs. A non-positive goal reports zero progress; values above 100% draw a second lap up to 200%. The last day is selected by default; legend hover and press highlight that ring and swap the center readout.

## API

`new` · `(title, goals, days, &mut Context<Self>)` selects the last day.

`set_day` · `(index, &mut Context<Self>)` and `set_days` · `(Vec<ActivityDay>, &mut Context<Self>)` retarget the rings from their current values on a 0.4s spring. `set_days` preserves the selected day by ID and falls back to the last day if that ID was removed.

Tab enters the selected day. Arrow keys move the selection and focus together. Home and End select the first or last day. Legend buttons highlight a goal.

`ActivityDayChanged` reports a date selection. `ActivityGoalClosed` fires when `set_days` raises a value on the selected date from below its goal to the goal or above.

## Layout

At a component width of 480px or more, the 216px dial sits beside the legend. Below 480px, the 184px dial sits above a full-width compact legend.

## Motion

Rings, per-ring markers and the center counter retarget from their current state; the center counter rolls in from zero when first revealed (0.48s wheel with per-digit stagger). Reduced motion updates directly.

Desktop and web use the same GPUI view.
