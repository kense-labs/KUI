# UsageMeter

Compare named allocations with a fixed capacity.

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
kui add usage-meter
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::usage_meter::{UsageMeter, Allocation};

let control = cx.new(|_| UsageMeter::new("Storage", "GB", 15.0, vec![Allocation::new("Photos", 4.0, gpui::rgb(0x7195ca).into())]));
```

## Interaction

Specify capacity, units and labeled allocations. Selecting a legend item emphasizes its segment.

## API

`set_usage` · `(f32, Vec<Allocation>)`

Specify capacity, units and labeled allocations. Selecting a legend item emphasizes its segment.

## Motion

On first paint the segments grow with 70ms offsets. Quota and amount changes retarget the displayed values and their widths together.

Desktop and web use the same GPUI view.
