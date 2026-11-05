# Treemap

Explore hierarchical values with a responsive squarified layout.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add treemap`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::treemap::{TreeTile, Treemap};

let children = vec![TreeTile::new("basic", "Basic", 40.0, gpui::rgb(0x8f7ad4).into()), TreeTile::new("team", "Team", 60.0, gpui::rgb(0x62b5d0).into())];
let tiles = vec![TreeTile::new("north", "North", 0.0, gpui::rgb(0x8f7ad4).into()).with_children(children)];
let control = cx.new(|_| Treemap::new("Business", tiles));
```

## Interaction

IDs must be unique within each level. An empty parent value uses the sum of its children.

## API

`set_tiles` · `(Vec<TreeTile>, &mut Context<Self>)`

Use with_metrics for sizes and with_shades for explicit colors. For numeric shading, use with_shade_values and with_color_scales; with_branch_hues preserves each branch's hue.

Event: `TreemapInspect`.

## Motion

Tiles zoom during drill-down, and matching IDs reflow when the metric changes; reduced motion updates directly.

Desktop and web use the same GPUI view.
