# Integration settings

Configure a connected SaaS service with a master sync switch, masked/revealable API key, live domain validation, searchable timezone picker, and an animated dirty-state save bar.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add integration-settings`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::integration_settings::{IntegrationSettings, IntegrationSettingsEvent};

let card = cx.new(|cx| {
    IntegrationSettings::new("stripe-integration", cx)
        .with_provider("stripe")
        .with_name("Stripe store")
        .with_count(Some(128))
});
cx.subscribe(&card, |_, event: &IntegrationSettingsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Toggle the capsule `pro_switch` to pause or resume sync, reveal or mask the API key `Editor`, copy or regenerate the key, type into `Site address` for real-time green/red domain validation, search and select a reporting timezone (`Escape` closes the popover), and confirm or revert edits from the `Discard` / `Save changes` footer.

## API

Primary GPUI type: `IntegrationSettings` (`kui::components::integration_settings::IntegrationSettings`).
Event type: `IntegrationSettingsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_provider(...)`, `with_name(...)`, `with_count(...)`, `with_meta(...)`, `with_site_hint(...)`, `with_values(...)`, `with_time_zones(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Toggling the master switch folds or reveals the configuration fields via `pro_fold`, editing any value slides the `Discard` / `Save changes` footer in and out via `pro_presence`, and `Theme::reduced_motion` settles both transitions immediately.

Desktop and web use the same GPUI source.
