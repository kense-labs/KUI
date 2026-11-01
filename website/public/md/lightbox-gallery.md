# PhotoExhibit

Open a larger image and navigate the rest of the gallery.

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
kui add lightbox-gallery
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::lightbox_gallery::{GalleryPhoto, PhotoExhibit};

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
