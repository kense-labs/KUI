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
use kui::components::race_bar_chart::{RaceBarChart, RaceEntry, RaceFrame};

let entries = vec![RaceEntry::new("first", "First"), RaceEntry::new("second", "Second")];
let frames = vec![
    RaceFrame::new("Q1", vec![("first".into(), 68.0), ("second".into(), 32.0)]),
    RaceFrame::new("Q2", vec![("first".into(), 54.0), ("second".into(), 72.0)]),
];
let control = cx.new(|_| RaceBarChart::new("Values", entries).with_frames(frames));
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
