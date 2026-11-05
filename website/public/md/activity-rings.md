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
let control = cx.new(|_| ActivityRings::new("Daily activity", goals, days));
```

## Interaction

Supply stable goal and day IDs. A non-positive goal reports zero progress; values above 100% draw a second lap up to 200%.

## API

`set_days` · `(Vec<ActivityDay>, &mut Context<Self>)`

Select a day with the radio group or set_day; legend buttons highlight a goal. Real-device touch, keyboard focus and screen-reader behavior remain to be checked.

Event: `ActivityDayChanged, ActivityGoalClosed`.

## Motion

Rings and values retarget from their current state; reduced motion updates directly. Visual frame comparison is pending.

Desktop and web use the same GPUI view.
