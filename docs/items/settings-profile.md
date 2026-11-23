# Settings profile

Edit personal avatar, first and last name, gated email change, role, timezone, week start, personal context, and inline password update.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-profile`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_profile::{SettingsProfileScreen, SettingsProfileEvent};

let screen = cx.new(|cx| SettingsProfileScreen::new("settings-profile", cx));
cx.subscribe(&screen, |_, event: &SettingsProfileEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click Change next to email to unlock editing, use Up/Down/Enter inside Role, Timezone, and Week-start comboboxes, and save or discard dirty edits.

## API

Primary GPUI type: `SettingsProfileScreen` (`kui::components::settings_profile::SettingsProfileScreen`).
Event type: `SettingsProfileEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
