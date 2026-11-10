# Product gallery

A product gallery with a hover magnifier, gliding highlights, and a fullscreen zoom viewer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add product-gallery`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::product_gallery::{ProductColor, ProductGallery, ProductPhoto, ProductSize};

let colors = vec![ProductColor::new(
    "olive",
    "Olive",
    gpui::rgb(0x6b6a4a).into(),
    vec![
        ProductPhoto::new("https://images.unsplash.com/photo-1", "Olive jacket, front"),
        ProductPhoto::new("https://images.unsplash.com/photo-2", "Olive jacket, back"),
    ],
)];
let sizes = vec![
    ProductSize::new("s", "S"),
    ProductSize::new("m", "M").unavailable_for(vec!["olive".into()]),
];
let control = cx.new(|cx| ProductGallery::new("Field jacket", colors, sizes, cx));
```

`with_header` and `with_footer` take closures for the name, price, and purchase area; `header` sits above the pickers and `footer` under them. A meta line under the footer lists sizes sold out in the color on show.

## Interaction

Drag or flick the photo track one photo at a time; edges rubber-band. The thumbnail rail is a tablist and swatches and sizes are radiogroups with roving arrows and Home/End — sold-out sizes are skipped. Enter or Space on the main photo opens the fullscreen viewer; Escape zooms out first, then closes, and focus returns to the photo. Clicking the viewer photo toggles 2.4× zoom, which pans after the pointer.

Selecting a color crossfades every photo, retargets the rail, and clears a size that is sold out in the new color (`onSizeChange` fires with `None`). Sold-out sizes stay visible, struck through and skipped.

The gallery stacks the media and details at container widths of 680px or less. At 520px or less, thumbnails move below the main photo.

## API

`set_color` · `(&str, &mut Context<Self>)` — switch color, resetting to its first photo.

`set_size` · `(&str, &mut Context<Self>)` — pick a size; sold-out entries refuse.

`set_index` · `(usize, &mut Context<Self>)` — step the photo track.

`set_viewer` · `(bool, &mut Window, &mut Context<Self>)` — open or close the fullscreen viewer.

Events: `ProductGalleryEvent::{Color, Size, Index, Viewer}`.

## Motion

The track springs to the photo on show (0.6s) and follows a drag one to one with rubber-band edges; one flick over 360 px/s moves one photo. The 176px hover lens rides a spring behind the pointer at 2.4×. Rail, swatch, and size highlights glide to the active option over 0.3s; color changes crossfade over 0.24s. The viewer grows out of the photo's own rectangle over 0.3s and returns to it on close; zoom scales on a spring over 0.42s. Reduced motion jumps every animation.

Desktop and web use the same GPUI view. The reference blurs photos while crossfading and counter-scales the viewer corners; GPUI has neither blur nor FLIP corner scaling, so KUI crossfades without blur and keeps a constant corner radius.
