# ChipGroup

Toggle compact filter choices in place.

## Install

```bash
kui add chip-group
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::chip_group::{ChipGroup, FormOption};

let control = cx.new(|cx| ChipGroup::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
```

## Interaction

Filter by stable option keys. Expand the overflow row for additional filters.

## API

`selected` · `() -> Vec<SharedString>`

Filter by stable option keys. Expand the overflow row for additional filters.

Event: `SelectionChanged`.

## Motion

Independent per-chip spring controls the drawing, check width, and selected fill.

Desktop and web use the same GPUI view.
