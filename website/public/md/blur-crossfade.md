# Blur crossfade

Swap content with a soft-focus blur while the container glides smoothly to the measured size of what comes next.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add blur-crossfade`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::{div, prelude::*};
use kui::components::blur_crossfade::{BlurCrossfade, BlurMorph};

let status_btn = BlurCrossfade::label("publish-status", "published", "Published")
    .with_duration_ms(420.);

let step_panel = BlurCrossfade::from_fn("step-body", "step-1", |_, _| {
    div().child("Dark mode is live for everyone.").into_any_element()
})
.with_morph(BlurMorph::Height)
.with_blur_px(10.);
```

## Interaction

Only the active incoming content remains interactive; leaving layers are inert and clipped only while the swap is in flight so focus rings and shadows stay visible at rest.

## API

Primary GPUI types: `BlurCrossfade`, `BlurMorph`, `BlurCrossfadeShowcase` (`kui::components::blur_crossfade`).
Event type: `None`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `BlurMorph::Height`, `BlurMorph::Both`, and `BlurMorph::None`, plus instant swaps when reduced motion is active.

## Motion

Outgoing content blurs, expands slightly (`1.0 -> 1.02`), and fades over `0.8x` duration while incoming content sharpens in from `0.98` along `cubic-bezier(0.22, 1.0, 0.36, 1.0)` as `Slide` springs glide the measured height (and width in `BlurMorph::Both`). Rapid consecutive updates retain only the latest leaving copy and current content; `Theme::reduced_motion` swaps immediately.

Desktop and web use the same GPUI source.
