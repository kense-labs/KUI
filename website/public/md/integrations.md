# Integrations

Connect 22 multicolor SaaS tools across Popular, Google, and Communication groups with verified badges, OAuth scope switches, and webhook status inspector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add integrations`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::integrations::{IntegrationsScreen, IntegrationsEvent};

let screen = cx.new(|cx| IntegrationsScreen::new("integrations", cx));
cx.subscribe(&screen, |_, event: &IntegrationsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press / to search integrations, Space to connect or disconnect, J/K + Enter or I to open the OAuth scope inspector, and + Request to suggest a connector.

## API

Primary GPUI type: `IntegrationsScreen` (`kui::components::integrations::IntegrationsScreen`).
Event type: `IntegrationsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
