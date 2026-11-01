# DonutChart

Inspect the contribution of each source to a whole.

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
kui add donut-chart
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::donut_chart::DonutChart;
use kui::primitives::plot::Sample;

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
