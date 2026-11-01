# Badge

Mark a status or category with a compact text label.

## Install

```bash
kui add badge
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::badge::{Badge, BadgeTone};

let control = cx.new(|_| Badge::new("In review", BadgeTone::Info));
```

## Interaction

Use a short label and tone to identify a status without relying solely on hue.

## API

`set` · `(label, BadgeTone)`

Use a short label and tone to identify a status without relying solely on hue.

## Motion

Changed text crosses over in place with a short directional rise. The foreground, fill and border transition between severity colors.

Desktop and web use the same GPUI view.
