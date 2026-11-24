# World map

Render a Natural Earth continental world map with one elevated landmass, a traveling pastel coastline band, and an interactive continent switcher pill.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add world-map`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::world_map::{Continent, WorldMap, WorldMapEvent};

let map = cx.new(|cx| {
    WorldMap::new("global-coverage-map", cx)
        .with_continent(Continent::Europe)
        .with_show_switcher(true)
});
cx.subscribe(&map, |_, ev: &WorldMapEvent, _| {
    let _ = ev;
})
.detach();
```

## Interaction

Click the top-left capsule switcher or use `ArrowUp`, `ArrowDown`, `Home`, `End`, `Enter`, and `Escape` keys to switch across `North America`, `South America`, `Europe`, `Africa`, `Asia`, and `Oceania`.

## API

Primary GPUI type: `WorldMap` (`kui::components::world_map::WorldMap`).
Event type: `WorldMapEvent` (`kui::components::world_map::WorldMapEvent`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `Continent` (`NorthAmerica`, `SouthAmerica`, `Europe`, `Africa`, `Asia`, `Oceania`), `with_continent(...)`, `with_show_switcher(...)`, `with_playing(...)`, `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Switching continents crossfades the previous landmass out (`0.32s`) while settling the newly selected continent (`0.94 -> 1.0` scale around its focal origin) and sweeping a multi-layered coral-to-periwinkle pastel band continuously around its main coastline on a `14s` loop; `Theme::reduced_motion` renders the highlighted continent and static coastline shine immediately.

Desktop and web use the same GPUI source.
