# BarChart

Compare categorical measurements on the same scale.

## Install

```bash
kui add bar-chart
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::bar_chart::BarChart;
use crate::ui::primitives::plot::Sample;

let control = cx.new(|_| BarChart::new("Activity", vec![Sample::new("Mon", 12.0), Sample::new("Tue", 18.0)]));
```

## Interaction

Read or compare labeled categories with a shared vertical scale.

## API

`set_samples` · `Vec<Sample>`

Read or compare labeled categories with a shared vertical scale.

Event: `InspectSample`.

## Motion

Bars rise in a short stagger on entry. Keyed bars retain position and height through range changes; removed bars shrink away.

Desktop and web use the same GPUI view.
