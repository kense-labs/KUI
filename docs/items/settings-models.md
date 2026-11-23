# Settings models

Configure workspace default and fallback models, EU and Global provider catalogs with per-model capsule switches, custom OpenAI endpoints, and BYOK keys.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-models`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_models::{SettingsModelsScreen, SettingsModelsEvent};

let screen = cx.new(|cx| SettingsModelsScreen::new("settings-models", cx));
cx.subscribe(&screen, |_, event: &SettingsModelsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Filter All, Chat, Image, and Custom models, toggle provider availability with capsule switches, reveal or test masked API keys, and add custom endpoints.

## API

Primary GPUI type: `SettingsModelsScreen` (`kui::components::settings_models::SettingsModelsScreen`).
Event type: `SettingsModelsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
