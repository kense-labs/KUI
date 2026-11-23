# Agent orchestration

Visualize a 17-agent delegation hierarchy on a zoomable dot canvas paired with a live trace waterfall, token ledger, and span inspector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-orchestration`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_orchestration::{AgentOrchestrationScreen, AgentOrchestrationEvent};

let screen = cx.new(|cx| AgentOrchestrationScreen::new("agent-orchestration", cx));
cx.subscribe(&screen, |_, event: &AgentOrchestrationEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Pan and zoom the topology, click any agent node or waterfall row to inspect Tasks, Context, and I/O, and approve or pause live branches.

## API

Primary GPUI type: `AgentOrchestrationScreen` (`kui::components::agent_orchestration::AgentOrchestrationScreen`).
Event type: `AgentOrchestrationEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
