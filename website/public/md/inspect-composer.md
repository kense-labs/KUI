# Inspect composer

Render an AI chat composer tilted in 3D perspective with Figma-style `#0190fe` selection chrome, corner handles, dimension badges, and a red spacing guide stepping through its regions.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add inspect-composer`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::inspect_composer::InspectComposer;

let composer = cx.new(|cx| {
    InspectComposer::new("hero-composer-inspect", cx)
        .with_mode("Auto")
        .with_hold(1600)
});
```

## Interaction

Automatically steps the pixel-inspection frame through `Message` (`14px`), `Toolbar` (`14px`), `Mode` selector (`8px`), and `Send & Apps` footer (`10px`). Hovering any region pins inspection via inverse 3D perspective hit-testing; clicking pins/unpins, arrow keys step across regions, and `Enter` / `Space` pauses or resumes playback.

## API

Primary GPUI type: `InspectComposer` (`kui::components::inspect_composer::InspectComposer`).
Supporting enum: `InspectComposerRegion`.
Event type: `InspectComposerEvent` (`PinRegion(Option<InspectComposerRegion>)`, `PlayingChanged(bool)`).

Supports light and dark `Theme` modes, `with_mirror(true)` left-fading default (`false` flips to right-fading), `with_flat(true)` zero-tilt mode, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`Slide` springs morph all four coordinates `(x, y, width, height)` of the `#0190fe` selection frame in 3D perspective while staggered reveal curves sequence the hand cursor, diagonal padding hatch, corner handles, layer tag, padding badges, `#ff3b30` spacing guide, and dimension pill; `Theme::reduced_motion` holds inspection steady on `Message`.

Desktop and web use the same GPUI source.
