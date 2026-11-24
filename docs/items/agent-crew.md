# Agent crew

Display a lead agent and its sub-agents as one persistent live tree of cards on soft trays, with rounded connectors, glowing handoff pulses, context-window meters, and per-agent steering.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-crew`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_crew::{AgentCrew, AgentCrewEvent};

let crew = cx.new(|cx| AgentCrew::new("agent-crew", cx));
cx.subscribe(&crew, |_, event: &AgentCrewEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch between projects in the top-left dropdown (`Fix the checkout crash`, `Q4 pricing rollout`, `Auth token migration`), click `Send handoff` to launch a light pulse along the rounded connectors into a destination card, or click any agent card to inspect its prompt, model, live usage, tools, and steering actions (`Pause`, `Resume`, `Stop`, `Approve`, `Decline`, `Retry`, and direct `Message` input).

## API

Primary GPUI type: `AgentCrew` (`kui::components::agent_crew::AgentCrew`).
Event type: `AgentCrewEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_root(...)`, `with_projects(...)`, `with_active_project(...)`, `with_handoffs(...)`, `with_auto_handoffs(...)`, `push_handoff(...)`, `steer_agent(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Active agents twinkle their 3×3 pixel marks in a diagonal wave while their activity line shimmers. Context bars glide via `pro_spring_bar` and shift from the branch color to orange at `80%` and red at `95%`. Handoffs travel hop-by-hop along `pro_connector_pulse` paths and wash the destination card with a directional `linear_gradient` glow; `Theme::reduced_motion` disables continuous pulses and settles handoffs immediately.

Desktop and web use the same GPUI source.
