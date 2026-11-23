# Settings general

Configure workspace icon, display name, URL slug, 2,000-character company context, dirty-state save bar, and name-verified workspace deletion.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-general`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_general::{SettingsGeneralScreen, SettingsGeneralEvent};

let screen = cx.new(|cx| SettingsGeneralScreen::new("settings-general", cx));
cx.subscribe(&screen, |_, event: &SettingsGeneralEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Editing name, slug, or context reveals the Save/Discard bar; Delete workspace requires typing the exact workspace name before confirmation unlocks.

## API

Primary GPUI type: `SettingsGeneralScreen` (`kui::components::settings_general::SettingsGeneralScreen`).
Event type: `SettingsGeneralEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
