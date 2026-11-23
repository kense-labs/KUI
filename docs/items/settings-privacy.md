# Settings privacy

Manage analytics, ad measurement, model-training opt-in, browser cookie categories, active device sessions, shared chat links, data export, and chat deletion.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-privacy`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_privacy::{SettingsPrivacyScreen, SettingsPrivacyEvent};

let screen = cx.new(|cx| SettingsPrivacyScreen::new("settings-privacy", cx));
cx.subscribe(&screen, |_, event: &SettingsPrivacyEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Toggle privacy preferences with capsule switches, copy or revoke shared chat links, sign out other devices while protecting This device, or export archive.

## API

Primary GPUI type: `SettingsPrivacyScreen` (`kui::components::settings_privacy::SettingsPrivacyScreen`).
Event type: `SettingsPrivacyEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
