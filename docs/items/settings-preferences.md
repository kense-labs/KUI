# Settings preferences

Choose default chat and image models with an interactive Auto Model Wheel, UI language, System/Light/Dark appearance, chat tools, instructions, and memory.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-preferences`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_preferences::{SettingsPreferencesScreen, SettingsPreferencesEvent};

let screen = cx.new(|cx| SettingsPreferencesScreen::new("settings-preferences", cx));
cx.subscribe(&screen, |_, event: &SettingsPreferencesEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch System, Light, and Dark appearance pills, toggle Beta actions, custom instructions, and memory with capsule switches, or forget individual memories.

## API

Primary GPUI type: `SettingsPreferencesScreen` (`kui::components::settings_preferences::SettingsPreferencesScreen`).
Event type: `SettingsPreferencesEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
