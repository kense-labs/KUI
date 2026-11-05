# Brush chart

Inspect a time series and drag an overview window to zoom.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add brush-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::brush_chart::{BrushChart, BrushPoint, BrushPreset};

let data = vec![BrushPoint::new("Day 1", 24.0), BrushPoint::new("Day 2", 32.0).with_event("Release")];
let control = cx.new(|_| BrushChart::new("Activity", data).with_presets(vec![BrushPreset::new("All", None)]));
```

## Interaction

Points are ordered by time. Window endpoints are inclusive indices.

## API

`set_window` · `(usize, usize, &mut Context<Self>)`

Drag the window or handles; keyboard users can pan and resize. Optional presets select the most recent points.

Event: `BrushChange, BrushInspect`.

## Motion

The line reveals on entry; controlled window changes and axis range use springs. Reduced motion updates directly.

Desktop and web use the same GPUI view.
