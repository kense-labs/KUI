# Sparkline

Inspect the shape of a compact time series.

## Install

```bash
kui add sparkline
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::sparkline::Sparkline;
use crate::ui::primitives::plot::Sample;

let control = cx.new(|_| Sparkline::new("Activity", vec![Sample::new("Mon", 12.0), Sample::new("Tue", 18.0)]));
```

## Interaction

Inspect a compact series with the pointer or arrow keys.

## API

`set_samples` · `Vec<Sample>`

Inspect a compact series with the pointer or arrow keys.

Event: `InspectSample`.

## Motion

The line is revealed across the plot on entry. A new range interpolates the sampled shape; the inspection marker moves between neighboring points.

Desktop and web use the same GPUI view.
