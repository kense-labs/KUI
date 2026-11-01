# LineChart

Compare reported and prior values across the same sample positions.

## Install

```bash
kui add line-chart
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::line_chart::LineChart;
use crate::ui::primitives::plot::Sample;

let control = cx.new(|_| LineChart::new("Activity", vec![Sample::new("Mon", 12.0), Sample::new("Tue", 18.0)]));
```

## Interaction

Compare a primary series with optional comparison values. Toggle the comparison and inspect aligned samples.

## API

`set_samples` · `Vec<Sample>`

Compare a primary series with optional comparison values. Toggle the comparison and inspect aligned samples.

Event: `InspectSample`.

## Motion

The initial line is revealed from left to right. A new range morphs the curve, comparison data fades independently, and the inspection cursor follows the pointer.

Desktop and web use the same GPUI view.
