# Agent usage

Forecast monthly agent spend against an editable budget cap with cumulative curve scrubbing, project/model splits, and anomaly callouts.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-usage`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_usage::{AgentUsageScreen, AgentUsageEvent};

let screen = cx.new(|cx| AgentUsageScreen::new("agent-usage", cx));
cx.subscribe(&screen, |_, event: &AgentUsageEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch 7d, 30d, and Billing cycle pills, scrub across days on the cumulative forecast chart, edit the monthly budget cap inline, and inspect daily bars.

## API

Primary GPUI type: `AgentUsageScreen` (`kui::components::agent_usage::AgentUsageScreen`).
Event type: `AgentUsageEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
