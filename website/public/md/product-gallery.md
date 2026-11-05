# Product gallery

Browse product photos and select a color and size.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add product-gallery`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::product_gallery::{ProductGallery, ProductColor, ProductPhoto, ProductSize};

let colors = vec![ProductColor::new("natural", "Natural", gpui::rgb(0xc9b99d).into(), vec![ProductPhoto::new("https://images.unsplash.com/photo-1500530855697-b586d89ba3ee", "Front view")])];
let sizes = vec![ProductSize::new("s", "S"), ProductSize::new("m", "M")];
let control = cx.new(|cx| ProductGallery::new("Product photos", colors, sizes, cx));
```

## Interaction

Select thumbnails, a color and an available size; open the image viewer for a closer look.

## API

`set_color` · `(&str, &mut Context<Self>)`

Sizes can declare sold-out colors; changing color clears an unavailable size selection.

Event: `ProductGalleryEvent`.

## Motion

Photo changes crossfade and the magnifier follows the pointer; reduced motion updates immediately.

Desktop and web use the same GPUI view.
