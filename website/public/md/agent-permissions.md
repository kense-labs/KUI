# Agent permissions

Set global autonomy posture, per-app read/write/risky rules, spending thresholds, and non-negotiable red lines.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-permissions`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_permissions::{AgentPermissionsScreen, AgentPermissionsEvent};

let screen = cx.new(|cx| AgentPermissionsScreen::new("agent-permissions", cx));
cx.subscribe(&screen, |_, event: &AgentPermissionsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Left/Right switches Careful, Balanced, and Hands-off presets, J/K selects apps, / filters rules, and inline spending caps commit on Enter.

## API

Primary GPUI type: `AgentPermissionsScreen` (`kui::components::agent_permissions::AgentPermissionsScreen`).
Event type: `AgentPermissionsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
