# NumberField

Enter a quantity directly or move it by one permitted step.

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
kui add number-field
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::number_field::NumberField;

let control = cx.new(|cx| NumberField::new("Seats", 8.0, 1.0..=50.0, 1.0, window, cx));
```

## Interaction

Type a number and press Enter or leave the field to clamp and commit; arrows change one step.

## API

`value` · `() -> f64`

Type a number and press Enter or leave the field to clamp and commit; arrows change one step.

Event: `f64`.

## Motion

Direct typing remains an editor. The clipped number rolls during stepper changes, with a bounded-limit nudge.

Desktop and web use the same GPUI view.
