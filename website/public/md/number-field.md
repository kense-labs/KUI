# NumberField

Enter a quantity directly or move it by one permitted step.

## Install

```bash
kui add number-field
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::number_field::NumberField;

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
