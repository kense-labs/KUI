# Waffle chart

Compare category shares in a 100-cell grid.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add waffle-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::waffle_chart::{WaffleCategory, WaffleChart};

let items = vec![WaffleCategory::new("search", "Search", 42.0, gpui::rgb(0x578ce2).into()), WaffleCategory::new("direct", "Direct", 58.0, gpui::rgb(0x47ab8b).into())];
let control = cx.new(|_| WaffleChart::new("Visitors", items));
```

## Interaction

Legend toggles filter categories; at least one remains visible. Arrow keys inspect grid cells.

## API

`set_categories` · `(Vec<WaffleCategory>, &mut Context<Self>)`

Supply stable category ids, labels, values and colors; subscribe to WaffleInspect for the active category.

Event: `WaffleInspect`.

## Motion

Cells retain identity and spring to their new positions; reduced motion places them immediately.

Desktop and web use the same GPUI view.
