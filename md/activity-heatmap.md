# DailyActivity

Explore activity day by day over a longer interval.

## Install

```bash
kui add activity-heatmap
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::activity_heatmap::DailyActivity;
use crate::ui::primitives::plot::Sample;

let control = cx.new(|_| DailyActivity::new("Activity", vec![Sample::new("Mon", 12.0), Sample::new("Tue", 18.0)]));
```

## Interaction

One column holds seven chronological days. Arrow keys move by day or week.

## API

`set_samples` · `Vec<Sample>`

set_first_weekday aligns the first sample to a weekday (Sunday = 0). Left/Right move by week, Up/Down by day.

Event: `InspectSample`.

## Motion

Cells appear diagonally across the calendar. Changes between years propagate by diagonal rank and retarget from the currently displayed intensity.

Desktop and web use the same GPUI view.
