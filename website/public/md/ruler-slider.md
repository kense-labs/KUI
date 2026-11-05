# Ruler slider

Choose a precise value by moving a tape under a fixed needle.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add ruler-slider`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ruler_slider::RulerSlider;

let control = cx.new(|cx| RulerSlider::new("Height", 120.0..=220.0, 1.0, 172.0, cx).with_unit("cm"));
```

## Interaction

Drag or tap the tape, use arrow keys, or scroll horizontally. Home and End go to the bounds.

## API

`with_unit` · `(impl Into<SharedString>) -> Self`

Set bounds, step, initial value, unit, major tick spacing and disabled state. RulerChanged reports changes; RulerCommitted reports releases and key or scroll actions.

Event: `RulerChanged / RulerCommitted`.

## Motion

The tape springs to a tick and changed digits roll; reduced motion removes the spring and digit animation.

The readout uses tabular numerals in persistent, fixed-advance columns with a stable 34px line box and baseline. Decimal precision follows the minimum and step: a 0.5 step retains one fractional digit, including `70.0`, so the decimal point and unit do not shift at whole values. Fast updates retarget each digit from its current position; a carry opens or closes only the new column's width. Rest and motion use the same layout.

Desktop and web use the same GPUI view.
