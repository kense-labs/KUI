# Gauge

Show a value against a known range with a named status.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add gauge`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::gauge::{Gauge, GaugeThreshold, GaugeTone};

let control = cx.new(|_| Gauge::new("Storage", 68.0).with_range(0.0, 100.0).with_detail("Used capacity").with_thresholds(vec![GaugeThreshold::new(80.0, "High", GaugeTone::Warning)]));
```

## Interaction

The host owns the value. The display clamps to the range; non-finite updates are ignored.

## API

`set_value` · `(f32, &mut Context<Self>)`

Use with_range, with_detail, with_tone and with_thresholds to configure the meter.

Event: `—`.

## Motion

The arc and displayed count follow one spring; reduced motion reaches the target immediately.

Desktop and web use the same GPUI view.
