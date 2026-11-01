# LineChart

Compare reported and prior values across the same sample positions.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add line-chart
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::line_chart::LineChart;
use kui::primitives::plot::Sample;

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
