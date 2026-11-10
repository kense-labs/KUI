# Photo grid

Browse square thumbnails, switch album density, and view or favorite individual photos.

## Install

Run `cargo add kense-kui --rename kui --no-default-features` and call `kui::init(cx)` before opening a window. To copy source, run `kui init` and `kui add photo-grid`, then import from `crate::ui`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::photo_grid::{GridPhoto, PhotoGrid};

let photo = GridPhoto::new("west", "west.jpg".to_owned(), "Sunset over a canyon", "West canyon", gpui::size(gpui::px(900.), gpui::px(600.)))
    .with_detail("October 9")
    .with_thumbnail("west-thumb.jpg".to_owned());
let grid = cx.new(|cx| PhotoGrid::new("October", vec![photo], cx));
```

## Options

| Rust option | Default and behavior |
| --- | --- |
| `GridPhoto` | Provide a unique stable ID, image source, alt text, title, and intrinsic size. An optional detail appears in the viewer. |
| `with_thumbnail` | Use a separate image for the grid and strip. The full source is loaded for the active viewer image. |
| `with_zoom_levels` | Defaults to 5, 3, 2 columns. Accepts unique counts from 1 to 12 and orders them from dense to spacious. An empty list keeps the defaults. |
| `with_default_columns` | Defaults to 3. Accepts 1 to 12 columns, including an initial count outside the zoom levels. Zooming steps to the nearest configured count in the requested direction. |
| `with_default_favorites` | Defaults to empty. Accepts known stable IDs. |
| `with_height` | Defaults to 380px, with a 320px minimum. Density stays explicit across widths. |
| `PhotoGridEvent` | `ColumnsChanged` on a committed density change. `FavoriteChanged` carries ID and new state. `ViewerChanged` reports the current ID and emits `None` at the start of dismissal. |

Arrow keys navigate the grid. Page Up and Page Down move by three rows. Home and End jump to the first and last photo. Use + or − to change density and Enter or Space to open. The viewer supports arrows, Home, End, and Escape, and restores focus to the photo last viewed.

Pinch or Control + scroll changes grid density. In the viewer, swipe horizontally or scrub the filmstrip to page, and pull down to dismiss. At widths below 480px, the strip and header become smaller and previous and next buttons are hidden. Reduced motion uses immediate reflow and fades without layout travel.
