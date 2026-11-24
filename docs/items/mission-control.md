# Mission control

Oversee every active agent at a glance—event peaks since you last looked on top, supervisor-to-worker hierarchy on the left, and live approvals on the right under one filter.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add mission-control`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::mission_control::{
    MissionControlBlock, MissionControlEvent, MissionFilter,
};

let control = cx.new(|cx| {
    MissionControlBlock::new("agent-mission-control", cx)
        .with_default_filter(MissionFilter::All)
});
cx.subscribe(&control, |_, event: &MissionControlEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch the unified top filter (`All`, `Running`, `Needs you`, `Failed`) or press Left/Right/Home/End to filter the `"Since you looked"` curve, `"Agents"` tree, and `"NEEDS YOU"` queue together. Hover any colored peak dot on the curve to highlight its agent across both lower panels, click a lead (`Launch lead`, `Support lead`, `Ops`) to inspect its connected sub-agents, and click `Approve`, `Reject`, `Retry`, or answer choices directly on the cards.

## API

Primary GPUI type: `MissionControlBlock` (`kui::components::mission_control::MissionControlBlock`).
Event type: `MissionControlEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation (`894px` down to `390px`).

## Motion

Sub-agent connectors animate traveling light pulses (`pro_connector_pulse`), status filter pills glide (`pro_pill_indicator`), working agent avatars twinkle (`pro_agent_avatar`), and resolving an action updates all three panels simultaneously. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
