# Skeleton

Reserve the shape of content while a view is loading.

## Install

```bash
kui add skeleton
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::skeleton::Skeleton;

let control = cx.new(|cx| Skeleton::new("Loading summary", [0.8, 1.0, 0.6], cx).with_avatar());
```

## Interaction

Provide an accessible loading label and relative line widths. Replace the placeholder when data arrives.

## API

`with_avatar` · `Self → Self`

Provide an accessible loading label and relative line widths. Replace the placeholder when data arrives.

## Motion

Placeholder blocks pulse in sequence every 1.8s, offset by 90ms. They stop after eleven cycles and remain static under reduced motion.

Desktop and web use the same GPUI view.
