# ChipGroup

Toggle compact filter choices in place.

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
kui add chip-group
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::chip_group::{ChipGroup, FormOption};

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
