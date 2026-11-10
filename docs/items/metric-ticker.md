# Metric ticker

Scan changing metrics and inspect their recent history.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add metric-ticker`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::metric_ticker::{MetricTicker, TickerMetric};

let metrics = vec![TickerMetric::new("requests", "Requests", 1234.0)];
let control = cx.new(|cx| MetricTicker::new("Live metrics", metrics, cx));
```

## Interaction

Use unique stable IDs. The host owns updates and visibility; non-finite values are ignored, and a zero baseline has no percentage change.

## API

`set_metrics` · `(Vec<TickerMetric>, &mut Context<Self>)`

Configure format, delta type, favorable direction and history per metric. Hover, focus or tap to inspect; use set_visible when outside the viewport. Touch and screen-reader behavior still require device testing.

`with_inline_deltas(false)` hides change badges in the compact strip while keeping trend details in the inspection panel.

Event: `TickerInspect`.

## Motion

Digits roll and the overflow strip drifts while visible; reduced motion disables automatic drift. Frame-by-frame visual comparison remains pending.

Desktop and web use the same GPUI view.
