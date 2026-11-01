# ColorPicker

Adjust a color visually or enter its precise hexadecimal value.

## Install

```bash
kui add color-picker
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::color_picker::{ColorPicker, ColorValue};

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
