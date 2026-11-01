# MultiSelect

Choose several options while retaining each selection by its stable key.

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
kui add multi-select
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::multi_select::{MultiSelect, FormOption};

let control = cx.new(|cx| MultiSelect::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
```

## Interaction

Search options, toggle without closing, or clear all.

Selected values use 24px-high labels with 8px corners and 8px horizontal padding. Long labels truncate at 144px. When selected labels wrap, the field grows to contain the next row.

## API

`selected` · `() -> Vec<SharedString>`

Search options, toggle without closing, or clear all.

Event: `SelectionChanged`.

## Motion

Selected labels are 24px high with 8px corners and wrap inside the trigger. Adding or removing a label animates the width around its measured text. The menu remains mounted while it closes.

Desktop and web use the same GPUI view.
