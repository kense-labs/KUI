# Slope chart

Compare paired measurements across two points.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add slope-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::slope_chart::{SlopeChart, SlopeSeries};

let series = vec![
    SlopeSeries::new("design", "Design", 40.0, 70.0),
    SlopeSeries::new("product", "Product", 55.0, 45.0),
];
let control = cx.new(|_| SlopeChart::new("Progress", series));
```

## Interaction

Hover a line, select its legend item or use arrow keys to inspect a series.

## API

`set_series` · `(Vec<SlopeSeries>, &mut Context<Self>)`

Provide stable series IDs, labels and paired values. Use `with_highlight` to name the series that starts highlighted. Subscribe to `SlopeInspect` to receive the values of the focused series.

Event: `SlopeInspect`.

## Motion

Existing series interpolate from their current positions when new values arrive.

Desktop and web use the same GPUI view.
