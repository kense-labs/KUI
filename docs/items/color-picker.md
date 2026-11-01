# ColorPicker

Adjust a color visually or enter its precise hexadecimal value.

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
kui add color-picker
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::color_picker::{ColorPicker, ColorValue};

let control = cx.new(|cx| ColorPicker::new("Accent", ColorValue(0x2F6BFF), window, cx));
```

## Interaction

Pointer and keyboard edit hue, saturation and brightness; valid hex input updates the same RGB value.

## API

`value` · `() -> ColorValue`

Pointer and keyboard edit hue, saturation and brightness; valid hex input updates the same RGB value.

Event: `ColorValue`.

## Motion

Height/width are spring-driven, the three color channels continuously retarget, and exit remains mounted.

Desktop and web use the same GPUI view.
