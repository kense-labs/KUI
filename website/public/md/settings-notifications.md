# Settings notifications

Control daily digest timing, weekly usage reports, per-event delivery channels across 8 activity types, and night-time quiet hours.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-notifications`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_notifications::{SettingsNotificationsScreen, SettingsNotificationsEvent};

let screen = cx.new(|cx| SettingsNotificationsScreen::new("settings-notifications", cx));
cx.subscribe(&screen, |_, event: &SettingsNotificationsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Toggle Daily digest, Weekly report, and Quiet hours capsule switches, bulk-assign channels via Notify on, or navigate comboboxes with Up/Down/Enter/Esc.

## API

Primary GPUI type: `SettingsNotificationsScreen` (`kui::components::settings_notifications::SettingsNotificationsScreen`).
Event type: `SettingsNotificationsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
