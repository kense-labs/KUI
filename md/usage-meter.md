# UsageMeter

Compare named allocations with a fixed capacity.

## Install

```bash
kui add usage-meter
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::usage_meter::{UsageMeter, Allocation};

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
