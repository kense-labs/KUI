# SegmentedControl

Switch a setting among a few adjacent segments.

## Install

```bash
kui add segmented-control
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::segmented_control::{SegmentedControl, FormOption};

let control = cx.new(|cx| SegmentedControl::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
```

## Interaction

Select a single view using pointer or arrow keys.

## API

`selected` · `() -> Vec<SharedString>`

Select a single view using pointer or arrow keys.

Event: `SelectionChanged`.

## Motion

Rapidly selecting a different segment keeps current position and velocity. GPUI regression samples 64ms and switchback.

Desktop and web use the same GPUI view.
