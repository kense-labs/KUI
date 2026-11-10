# Cover flow

Browse a short photo collection on a perspective rail.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy the source, run `kui init` and `kui add cover-flow`, then use `crate::ui::` imports in the example.

## Usage

```rust
use gpui::prelude::*;
use kui::components::cover_flow::{CoverFlow, CoverFlowItem};

let items = vec![
    CoverFlowItem::new("ridge", "Ridge Loop", "ridge.jpg".to_owned())
        .with_subtitle("Coastal Range")
        .with_meta("8 km"),
    CoverFlowItem::new("lake", "Quiet Lake", "lake.jpg".to_owned()),
];
let rail = cx.new(|cx| CoverFlow::new("Autumn hikes", items, cx));
```

## Interaction

Click an adjacent cover or a position tick. Drag horizontally to browse or release with momentum. Sideways trackpad scroll follows the swipe and settles on a card. Vertical touch gestures are available to the surrounding page. Focus the rail and press Left or Right to step, or Home or End to jump to an edge.

## API

- `CoverFlowItem<T>` holds a stable ID, title, optional subtitle and metadata, `ImageSource`, alt text and host data. Alt defaults to the title. `with_image_position(x, y)` accepts normalized crop alignment and defaults to `(0.5, 0.5)`. `with_data(value)` sets the typed caption payload.
- `with_default_index(usize)` sets the starting card. The default is zero. `with_index(usize)` enables host control. Subscribe to `CoverFlowIndexChanged(usize)` and accept the requested index with `set_index(usize, &mut Context<Self>)`. `index()` returns the current accepted index.
- `with_caption` replaces the default title and subline. It receives the item, index, window and app context.
- `with_card_width(fn(f32) -> f32)` overrides card width using the measured rail width in logical pixels. The default is `clamp(168, rail_width × 0.38, 272)`, limited further in containers narrower than the card.

## Motion

The rail applies edge resistance and a release-velocity spring. Visible cards are drawn back to front with a 4:5 crop, projected side planes, image drift and a fading reflection. Reduced motion removes turn and drift, steps on a 40px horizontal drag or a horizontal swipe, and skips the spring. Rendered and reflected textures are prepared on first use. The sample uses local public-domain photographs and illustrative route statistics.

Desktop and web use the same GPUI view.
