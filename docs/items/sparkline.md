# Sparkline

Inspect the shape of a compact time series.

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
kui add sparkline
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::sparkline::Sparkline;
use kui::primitives::plot::Sample;

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
