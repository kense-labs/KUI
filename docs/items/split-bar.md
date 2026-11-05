# Split bar

Inspect and compare changing data.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add split-bar`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::split_bar::{SplitBar, SplitSegment};

let data = vec![SplitSegment::new("first", "First", 68.0, gpui::rgb(0x9d83e8).into()), SplitSegment::new("second", "Second", 32.0, gpui::rgb(0x62b5d0).into())];
let control = cx.new(|_| SplitBar::new("Values", data));
```

## Interaction

Use stable unique IDs. Non-finite and negative values are treated as zero.

## API

`set_segments` · `(Vec<SplitSegment>, &mut Context<Self>)`

Provide stable ids, labels, values and colors; with_description and with_unit configure the header. Call set_segments to update data and set_visible when the host hides the chart.

Event: `SplitInspect`.

## Motion

Inspection expands a segment while the others give way proportionally. Edges and fitted labels retarget their current spring trajectories; hidden charts stop requesting animation frames.

Desktop and web use the same GPUI view.
