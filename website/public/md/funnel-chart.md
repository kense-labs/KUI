# Funnel chart

Inspect conversion across ordered stages and compare two segments.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add funnel-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::funnel_chart::{FunnelChart, FunnelSegment, FunnelStage};

let segments = vec![FunnelSegment::new("mobile", "Mobile"), FunnelSegment::new("desktop", "Desktop")];
let stages = vec![FunnelStage::new("visit", "Visit", vec![600.0, 400.0]), FunnelStage::new("buy", "Buy", vec![120.0, 80.0])];
let control = cx.new(|cx| FunnelChart::new("Checkout", stages, segments, cx));
```

## Interaction

Stages use stable IDs. Values follow segment order; invalid and negative values count as zero.

## API

`set_stages` · `(Vec<FunnelStage>, &mut Context<Self>)`

Select a segment or compare two segments; inspect stages with pointer, touch or arrow keys. Call set_visible(false, cx) when the embedding host hides the chart; particle phase resumes without a catch-up jump. with_flow and set_flow control the optional particles.

Event: `FunnelInspect`.

## Motion

Bands morph from their current shape. Flow stops outside the viewport or under reduced motion.

Desktop and web use the same GPUI view.
