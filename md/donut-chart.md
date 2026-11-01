# DonutChart

Inspect the contribution of each source to a whole.

## Install

```bash
kui add donut-chart
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::donut_chart::DonutChart;
use crate::ui::primitives::plot::Sample;

let control = cx.new(|_| DonutChart::new("Activity", vec![Sample::new("Mon", 12.0), Sample::new("Tue", 18.0)]));
```

## Interaction

Inspect a slice or its legend entry to read its contribution.

## API

`set_samples` · `Vec<Sample>`

Inspect a slice or its legend entry to read its contribution.

Event: `InspectSample`.

## Motion

Segments sweep from twelve o’clock with delayed trailing edges. Range changes and toggling a legend row retarget live values, and inspecting a slice lifts it radially.

Desktop and web use the same GPUI view.
