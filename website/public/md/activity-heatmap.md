# DailyActivity

Explore activity day by day over a longer interval.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add activity-heatmap
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::activity_heatmap::DailyActivity;
use kui::primitives::plot::Sample;

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
