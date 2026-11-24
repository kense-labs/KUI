# Agent fleet

Compare several parallel lead agents in a compact column where each card summarizes its sub-agents, rolled-up branch cost, and urgent status, and clicking a lead unfolds its sub-agent tree smoothly to the right.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-fleet`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_fleet::{AgentFleet, AgentFleetEvent};

let fleet = cx.new(|cx| AgentFleet::new("agent-fleet", cx));
cx.subscribe(&fleet, |_, event: &AgentFleetEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click any lead agent card (or navigate with `Up`/`Down` and press `Enter`) to unfold its sub-agent hierarchy to the right while dimming unselected leads and folding the previously open tree. Click any sub-agent card (or its `Answer` / `Resolve` pill) to unfold its ordered task steps and approve, reject, retry, or dismiss pending gates inline.

## API

Primary GPUI type: `AgentFleet` (`kui::components::agent_fleet::AgentFleet`).
Event type: `AgentFleetEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_agents(...)`, `with_default_open(...)`, `set_open(...)`, `resolve_sub_agent(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Unselected lead cards dim smoothly via `Slide` springs (`0.50` resting, `0.86` hovered), sub-agent cards stagger in from the left as rounded orthogonal connectors draw out from the active lead card, rolled-up costs animate via `pro_rolling_number`, and sub-agent task drawers expand with `pro_fold`. Respects `Theme::reduced_motion` and adapts to narrow viewports by unfolding trees beneath their lead card.

Desktop and web use the same GPUI source.
