# Agent state machine

Visualize an autonomous agent as a live finite-state diagram (`Planning` → `Searching` → `Coding` → `Testing` → `Waiting` → `Done` bullseye)—with forward arrows, top self-retry loops, bottom loopback paths, rolling traversal counts, and dwell-time inspectors.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-state-machine`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_state_machine::{AgentStateMachine, AgentStateMachineEvent};

let machine = cx.new(|cx| AgentStateMachine::new("agent-state-machine", cx));
cx.subscribe(&machine, |_, event: &AgentStateMachineEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover any state card to highlight its incoming and outgoing transitions while dimming unrelated paths. Click `Step state` to watch a pulse travel along the active `PathBuilder` transition and roll its `N×` count upward, click any `N×` badge to inspect or fire that trigger, or click any state card to inspect its live dwell time, visit count, tools, and `Approve` / `Reject` actions.

## API

Primary GPUI type: `AgentStateMachine` (`kui::components::agent_state_machine::AgentStateMachine`).
Event type: `AgentStateMachineEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_agent(...)`, `with_states(...)`, `with_transitions(...)`, `with_current(...)`, `fire_transition(...)`, `resolve_waiting_state(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Active transitions carry an arc-length-parameterized comet pulse while their `N×` count pills roll upward via `pro_rolling_number`; the active state twinkles its pixel mark in the header and ticks its dwell timer live. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
