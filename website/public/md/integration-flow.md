# Integration flow

Render integration pills rotating along a dashed elliptical arc and beaming animated light pulses into a central Kense analysis hub.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add integration-flow`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::integration_flow::IntegrationFlow;

let flow = cx.new(|cx| {
    IntegrationFlow::new("hero-integrations", cx)
        .with_hub_title("Data Analysis")
});
```

## Interaction

Self-playing `2400 : 1490` stage loop: hovering any orbiting integration pill illuminates its curved inward beam and reveals its color mark; clicking or pressing `Enter` / `Space` pauses or resumes playback (`IntegrationFlowEvent::PausedChanged`).

## API

Primary GPUI type: `IntegrationFlow` (`kui::components::integration_flow::IntegrationFlow`).
Supporting item type: `IntegrationItem`.
Event type: `IntegrationFlowEvent` (`PausedChanged(bool)`, `Select(SharedString)`).

Supports light and dark `Theme` modes, custom integration lists (`with_items`), custom stage and bottom-fade colors (`with_background`, `with_fade_color`), bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states. Uses the Kense mark inside the central hub.

## Motion

Integration capsules advance slot by slot (`0.9s` eased arc transition) along a dashed elliptical `PathBuilder` orbit; at the center slot, each capsule charges up and sweeps a glowing `1.1s` pulse into the Kense hub over a `16s` breathing coral radial backdrop. `Theme::reduced_motion` holds the active center state and beam steady.

Desktop and web use the same GPUI source.
