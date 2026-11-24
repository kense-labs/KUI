# Brand regions

Inspect where a brand appears in AI answers country by country on a zoomable choropleth world map paired with ranked regional metrics.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add brand-regions`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::brand_regions::{BrandRegionsEvent, BrandRegionsScreen};

let screen = cx.new(|cx| BrandRegionsScreen::new("brand-regions", cx));
cx.subscribe(&screen, |_, event: &BrandRegionsEvent, _| {
    let _ = event;
})
.detach();
```

## Interaction

Use Left/Right to move across brand insight tabs, time windows (`7d`, `15d`, `30d`, `Custom`), and regional measures (`Visibility`, `Share of Voice`, `Sentiment`). Tab into the ranked region list and press Up/Down/Home/End to highlight countries on the map with their breakdown card, or press Enter (or click a country) to zoom to its continent and Enter again for the full world.

## API

Primary GPUI type: `BrandRegionsScreen` (`kui::components::brand_regions::BrandRegionsScreen`).
Event type: `BrandRegionsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` breathing map and row skeletons, `with_empty(true)` empty states, `with_zoom(...)` continental viewports (`World`, `Americas`, `Europe`, `Asia`), and responsive desktop-to-mobile adaptation.

## Motion

Sliding tab underlines, segmented pill indicators, country hover dimming, and loading map breathing follow KUI motion curves and respect `Theme::reduced_motion`. Responsive breakpoints stack the ranked list cleanly beneath the world map on narrower viewports.

Desktop and web use the same GPUI source.
