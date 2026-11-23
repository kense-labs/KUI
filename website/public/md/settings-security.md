# Settings security

Configure verified email domains, domain auto-join, SAML SSO identity provider metadata, 2FA enforcement, session duration, CIDR IP allowlist, and global sign-out.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add settings-security`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::settings_security::{SettingsSecurityScreen, SettingsSecurityEvent};

let screen = cx.new(|cx| SettingsSecurityScreen::new("settings-security", cx));
cx.subscribe(&screen, |_, event: &SettingsSecurityEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Add and verify domains to unlock Join by domain and SAML SSO switches, copy Audience/ACS URLs, validate IPv4/IPv6 CIDR ranges, and confirm Sign out everyone.

## API

Primary GPUI type: `SettingsSecurityScreen` (`kui::components::settings_security::SettingsSecurityScreen`).
Event type: `SettingsSecurityEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
