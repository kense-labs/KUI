# Elastic slider

Adjust one value using an elastic track and value bubble.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add elastic-slider`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::elastic_slider::ElasticSlider;

let control = cx.new(|cx| ElasticSlider::new("Volume", 0.0..=100.0, 1.0, 50.0, "%", window, cx));
```

## Interaction

Drag, click the rail or use arrow keys to change the value.

## API

`set_value` · `(f64, &mut Context<Self>)`

Supply a range, step, starting value and suffix. ElasticChanged emits the current value.

Event: `ElasticChanged`.

## Motion

The thumb strains past a bound and springs back; reduced motion suppresses the stretch.

Desktop and web use the same GPUI view.
