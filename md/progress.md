# Progress

Display measured completion or an indeterminate operation.

## Install

```bash
kui add progress
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::progress::Progress;

let control = cx.new(|cx| Progress::new("Preparing workspace", Some(0.68), cx));
```

## Interaction

Pass a fraction from zero to one, or None for indeterminate work.

## API

`set_value` · `Option<f32>`

Pass a fraction from zero to one, or None for indeterminate work.

## Motion

The label follows the same spring as the fill. The completion color and check follow the advance.

Desktop and web use the same GPUI view.
