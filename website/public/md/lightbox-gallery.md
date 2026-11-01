# PhotoExhibit

Open a larger image and navigate the rest of the gallery.

## Install

```bash
kui add lightbox-gallery
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::lightbox_gallery::{GalleryPhoto, PhotoExhibit};

let control = cx.new(|cx| PhotoExhibit::new(vec![GalleryPhoto::new("photos/harbor.jpg","Harbor","Dawn on the coast",1.5)],cx));
```

## Interaction

Open a tile, use Left/Right, swipe or zoom. Escape and vertical drag close the viewer.

## API

`open` · `(&mut self, index: usize, &mut Window, &mut Context<Self>)`

Supply ImageSource, title, caption and thumbnail aspect ratio per photo.

Event: `GallerySelection`.

## Motion

The photo flies from its current thumbnail with a stable corner radius. Directional changes slide through adjacent photos; reduced motion fades the viewer in place.

Desktop and web use the same GPUI view.
