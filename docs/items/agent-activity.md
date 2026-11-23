# Agent activity

Audit every action an agent took across connected apps, why it was allowed, and undo reversible steps.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-activity`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_activity::{AgentActivityScreen, AgentActivityEvent};

let screen = cx.new(|cx| AgentActivityScreen::new("agent-activity", cx));
cx.subscribe(&screen, |_, event: &AgentActivityEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Use J/K or Up/Down to step through rows, Enter to inspect on the right, U to undo a reversible step, Left/Right to switch filters, and / to search.

## API

Primary GPUI type: `AgentActivityScreen` (`kui::components::agent_activity::AgentActivityScreen`).
Event type: `AgentActivityEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
