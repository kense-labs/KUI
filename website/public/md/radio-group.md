# RadioGroup

Choose exactly one alternative in a labeled list.

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
kui add radio-group
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::radio_group::{RadioGroup, FormOption};

let control = cx.new(|cx| RadioGroup::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
```

## Interaction

Use one labelled choice for mutually exclusive options.

## API

`selected` · `() -> Vec<SharedString>`

Use one labelled choice for mutually exclusive options.

Event: `SelectionChanged`.

## Motion

One plate follows continuous row position. Old/new dots have independent springs.

Desktop and web use the same GPUI view.
