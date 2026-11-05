# Carousel

Navigate a collection of images by dragging or using controls.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add carousel`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::carousel::{Carousel, CarouselSlide};

let slides = vec![
    CarouselSlide::new("https://images.unsplash.com/photo-1500530855697-b586d89ba3ee", "Landscape"),
    CarouselSlide::new("https://images.unsplash.com/photo-1470770841072-f978cf4d019e", "Lake"),
];
let control = cx.new(|cx| Carousel::new("Gallery", slides, cx));
```

## Interaction

Drag horizontally or use the arrow keys, Home, End and navigation buttons.

## API

`set_index` · `(usize, &mut Context<Self>)`

Supply images and labels as CarouselSlide values; with_action_labels localizes navigation.

Event: `CarouselChanged(usize)`.

## Motion

Dragging follows the pointer; release settles to a slide using a spring. Reduced motion changes immediately.

Desktop and web use the same GPUI view.
