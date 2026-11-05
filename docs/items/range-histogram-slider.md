# Range histogram slider

Filter a numeric range against a distribution.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add range-histogram-slider`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::range_histogram_slider::RangeHistogramSlider;

let data = vec![30.0, 80.0, 180.0, 260.0, 400.0];
let control = cx.new(|cx| RangeHistogramSlider::new("Price", data, 0.0..=500.0, 5.0, (80.0, 260.0), 32, window, cx));
```

## Interaction

Drag either thumb, click the histogram or rail, or focus a thumb and use arrow keys. The selected count updates as the range changes.

## API

`value` · `(&App) -> (f64, f64)`

Supply raw values, bounds, step, initial range and bin count. HistogramRangeChanged reports both bounds. Release and chart clicks snap to the nearest bin edge, rounded to the configured step.

Event: `HistogramRangeChanged`.

## Motion

The shared slider provides elastic dragging and reduced-motion behavior. Bars update as values change.

The header range is a static, right-aligned readout using tabular numerals and a fixed 20px line height. It updates directly with the selected values; it does not switch between rolling character cells and ordinary text. Equal-length amounts keep the same position while dragging and after release.

Desktop and web use the same GPUI view.
