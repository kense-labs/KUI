# Settings subscription

Review current Pro plan, upcoming bill breakdown, 4 usage meters, capsule Monthly/Yearly plan switcher, add-ons, Visa payment details, and invoices.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-subscription`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_subscription::{SettingsSubscriptionScreen, SettingsSubscriptionEvent};

let screen = cx.new(|cx| SettingsSubscriptionScreen::new("settings-subscription", cx));
cx.subscribe(&screen, |_, event: &SettingsSubscriptionEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Toggle Monthly and Yearly -20% capsule pills, expand Compare plans or Upcoming bill details, edit billing address/card inline, and download PDF invoices.

## API

Primary GPUI type: `SettingsSubscriptionScreen` (`kui::components::settings_subscription::SettingsSubscriptionScreen`).
Event type: `SettingsSubscriptionEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
