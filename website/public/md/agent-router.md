# Agent router

Sort incoming customer requests through a central router agent to specialist queues (`Billing`, `Tech support`, `Sales`) and an off-to-the-right `Human` fallback—showing live connector pulses, confidence scores, and one-click manual rerouting.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-router`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_router::{AgentRouter, AgentRouterEvent};

let router = cx.new(|cx| AgentRouter::new("agent-router", cx));
cx.subscribe(&router, |_, event: &AgentRouterEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click `+ New request` to watch a new message travel down into `Router`, compute a confidence percentage, and either light up the matching specialist card and drop into its `Task queue` below or escalate low-confidence requests out right to `Human`. Click any request card to inspect every agent's classification score and manually `Reroute` it elsewhere.

## API

Primary GPUI type: `AgentRouter` (`kui::components::agent_router::AgentRouter`).
Event type: `AgentRouterEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_router(...)`, `with_agents(...)`, `with_requests(...)`, `reroute_request(...)`, `simulate_incoming(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Light pulses run from the incoming card into `Router` and onward along `pro_connector_pulse` S-bends to wash the chosen specialist card with a directional `linear_gradient` glow; `Router` twinkles its pixel mark and shimmers its reading status while confidence meters glide via `pro_spring_bar`. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
