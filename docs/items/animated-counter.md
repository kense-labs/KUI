# Animated counter

Give a changing total a directional, place-keyed digit roll.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add animated-counter`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::animated_counter::AnimatedCounter;

let control = cx.new(|_| AnimatedCounter::new(12540.25).with_label("Revenue").with_prefix("$").with_decimals(2));
```

## Interaction

The host supplies values. Non-finite updates are ignored; formatting uses comma grouping and up to six decimal places.

## API

`set_value` · `(f64, &mut Context<Self>)`

Use with_label, with_prefix, with_suffix, with_decimals and with_size. Opt into a first reveal with with_animate_on_view(true); it starts at 60% visibility. Hosts call set_visible when hiding the control.

Event: `—`.

## Motion

Digits retain their place and spring toward the updated value; reduced motion updates immediately.

Desktop and web use the same GPUI view.
