# Tilt card

Tilt a showcase card in 3D perspective toward the pointer with a soft radial glare and an underdamped spring recovery.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add tilt-card`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::{prelude::*, px};
use kui::components::tilt_card::TiltCard;

let card = TiltCard::new("member-card", cx)
    .with_max_tilt(10.0)
    .with_scale(1.02)
    .with_glare(true)
    .with_perspective(900.0)
    .with_radius(px(20.));
```

## Interaction

Purely visual hover enhancement that adds no extra tab stops while elevating slightly when any inner focusable control receives keyboard focus; touch pointers skip tilt and reduced motion restricts hover feedback to shadow elevation.

## API

Primary GPUI types: `TiltCard`, `TiltCardShowcase` (`kui::components::tilt_card`).
Event type: `None`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), configurable `max_tilt`, `glare`, `scale`, `perspective`, `radius`, and custom child content via `with_child` or `with_content`, plus the built-in KUI Pro membership card showcase.

## Motion

Pointer coordinates inside the card drive snappy `140ms` pitch/yaw springs, `1.02x` hover scale, dynamic opposing drop shadows, and a clipped soft-light radial glare. Leaving the card switches to a `700ms` underdamped spring (`zeta = 0.56`) that settles back to rest with a subtle overshoot; `Theme::reduced_motion` disables 3D rotation and glare immediately.

Desktop and web use the same GPUI source.
