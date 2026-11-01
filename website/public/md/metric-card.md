# MetricCard

Read one metric in the context of its change and comparison period.

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
kui add metric-card
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::metric_card::MetricCard;

let control = cx.new(|_| MetricCard::new("Active users", 2486.0, 18.4, "Previous 7 days"));
```

## Interaction

Pass the absolute value, percentage change and the comparison period.

## API

`update_metric` · `(f32, f32, comparison)`

Pass the absolute value, percentage change and the comparison period.

## Motion

Decimal places roll independently with a stagger on entry. Metric changes retarget the wheels, while the comparison and description crossfade.

Desktop and web use the same GPUI view.
