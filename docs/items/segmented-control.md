# SegmentedControl

Switch a setting among a few adjacent segments.

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
kui add segmented-control
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::segmented_control::{SegmentedControl, FormOption};

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
