# Image compare

Compare two images using a movable divider.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add image-compare`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::image_compare::ImageCompare;

let control = cx.new(|cx| ImageCompare::new(
    "https://images.unsplash.com/photo-1500530855697-b586d89ba3ee",
    "https://images.unsplash.com/photo-1470770841072-f978cf4d019e",
    "Compare images", cx,
));
```

## Interaction

Drag the divider, or use arrows, Home and End. Shift changes in larger steps.

## API

`with_position` · `(f32) -> Self`

Position is a fraction from 0 to 1; images are supplied by the host.

Event: `CompareChanged(f32)`.

## Motion

Keyboard and host updates settle with a spring; pointer and touch track input.

Desktop and web use the same GPUI view.
