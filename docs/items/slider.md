# Slider

Adjust a number or the ends of a numeric interval along a track.

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
kui add slider
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::slider::{Slider, SlideValue};

let control = cx.new(|cx| Slider::new("Budget", 0.0..=100.0, 1.0, SlideValue { low: 20.0, high: Some(80.0) }, cx));
```

## Interaction

Drag the nearest stop. Two stops cannot cross; Shift + arrow moves ten steps.

## API

`value` · `() -> SlideValue`

Drag the nearest stop. Two stops cannot cross; Shift + arrow moves ten steps.

Event: `SlideValue`.

## Motion

Drag past either stop to feel up to 9px of resistance. Release carries the visible velocity into the return spring. Reduced motion clamps directly.

Desktop and web use the same GPUI view.
