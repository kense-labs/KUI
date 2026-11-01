# BarChart

Compare categorical measurements on the same scale.

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
kui add bar-chart
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::bar_chart::BarChart;
use kui::primitives::plot::Sample;

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
