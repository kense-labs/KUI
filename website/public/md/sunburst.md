# Sunburst

Explore hierarchical values as nested rings.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add sunburst`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::sunburst::{Sunburst, SunburstNode};

let root = SunburstNode::new("all", "All teams", 0.0, gpui::rgb(0x8f7ad4).into()).with_children(vec![SunburstNode::new("platform", "Platform", 40.0, gpui::rgb(0x8f7ad4).into())]);
let control = cx.new(|_| Sunburst::new("Cloud spend", root));
```

## Interaction

Node IDs must be unique. Branch values are sums of children; non-finite and negative leaf values contribute zero.

## API

`with_value_format(prefix, suffix, decimals)` formats totals in host display units. `set_header(subtitle, extra, cx)` supplies an optional period label and filter view. The legend appears below the chart.

`set_root` · `(SunburstNode, &mut Context<Self>)`

Use arrow keys to select, Enter to focus, and Escape to return. Each label is shaped as one horizontal line so the text keeps its reading order. Labels are hidden when a segment is too small.

Event: `SunburstFocus`.

## Motion

Arc geometry and center digits transition between levels; reduced motion updates directly.

Desktop and web use the same GPUI view.
