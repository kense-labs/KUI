# Race bar chart

Inspect and compare changing data.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add race-bar-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::race_bar_chart::{RaceBarChart, RaceEntry};

let data = vec![RaceEntry::new("first", "First", 68.0, gpui::rgb(0x9d83e8).into()), RaceEntry::new("second", "Second", 32.0, gpui::rgb(0x62b5d0).into())];
let control = cx.new(|_| RaceBarChart::new("Values", data));
```

## Interaction

Use stable unique IDs. Non-finite and negative values are treated as zero.

## API

`set_entries` · `(Vec<RaceEntry>, &mut Context<Self>)`

The host supplies and updates values. Use the keyboard or pointer to inspect.

Event: `RaceInspect`, `RaceFrameChanged`, `RacePlaybackChanged`.

## Motion

Stable IDs animate from their current positions to updated values. Reduced motion updates immediately.

Desktop and web use the same GPUI view.
