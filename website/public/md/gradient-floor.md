# Gradient floor

Render a two-layer blurred color and additive light crest that drifts and morphs along the bottom edge.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add gradient-floor`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::{prelude::*, px};
use kui::components::gradient_floor::GradientFloor;

let hero = GradientFloor::new("hero-floor")
    .with_height(px(360.))
    .with_speed(3.0)
    .with_overlay(|_, _| gpui::div().child("Ideas start to glow").into_any_element());
```

## Interaction

Purely decorative backdrop hidden from assistive focus; pauses automatically when paused or when reduced motion is active.

## API

Primary GPUI type: `GradientFloor` (`kui::components::gradient_floor::GradientFloor`).
Event type: `None`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Two separable-Gaussian-blurred layers—a 3-stop color wave and a plus-lighter white crest—oscillate across 5 harmonic control points with continuous Fourier lateral drift. Frames evaluate at real-time vsync phase with subpixel vertical coverage and replace a single sprite-atlas tile in place; `Theme::reduced_motion` freezes the resting crest immediately.

Desktop and web use the same GPUI source.
