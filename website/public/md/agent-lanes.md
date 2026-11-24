# Agent lanes

Monitor a team of autonomous agents in parallel on a live horizontal swimlane timeline with tasks growing into `"now"`, curved dashed handoff connectors, and inline `Approve` / `Reject` actions.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-lanes`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_lanes::{AgentLanes, AgentLanesEvent};

let lanes = cx.new(|cx| {
    AgentLanes::new("agent-control-room", cx)
        .with_span(30 * 60_000)
        .with_live(true)
});
cx.subscribe(&lanes, |_, ev: &AgentLanesEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Waiting task blocks surface inline `Reject` and `Approve` actions directly on the block (and open a full decision dialog with the `ask` prompt when clicked or activated via `Enter` / `Space`). Clicking any completed or running block opens a summary popover listing completed bullet points and an `Open run →` action. Hovering an agent row highlights that lane across the timeline.

## API

Primary GPUI type: `AgentLanes` (`kui::components::agent_lanes::AgentLanes`).
Supporting types: `LaneAgent`, `LaneTask`, `LaneTaskStatus`.
Event type: `AgentLanesEvent` (`Approve(SharedString)`, `Reject(SharedString)`, `OpenTask(SharedString)`, `SelectTask(Option<SharedString>)`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeletons, and `with_empty(true)` empty states.

## Motion

Active tasks widen smoothly with the sliding `"now"` window, live handoff splines flow across lanes with animated dash phase (`PathBuilder`), and waiting indicators pulse softly; `Theme::reduced_motion` freezes continuous motion cleanly.

Desktop and web use the same GPUI source.
