# Ridgeline

Compare distributions and inspect their quartiles.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add ridgeline`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ridgeline::{Ridgeline, RidgeSeries};

let series = vec![RidgeSeries::new("jan", "January", vec![1.0, 3.0, 5.0], gpui::rgb(0x609cd8).into())];
let control = cx.new(|_| Ridgeline::new("Temperatures", series));
```

## Interaction

Non-finite samples are ignored; IDs stay stable between datasets.

## API

`set_series` · `(Vec<RidgeSeries>, &mut Context<Self>)`

Use pointer, touch or arrow keys to inspect a ridge and its quartiles.

Event: `RidgeInspect`.

## Motion

Curves interpolate on a shared grid and the selected ridge lifts. Reduced motion updates directly.

Desktop and web use the same GPUI view.
