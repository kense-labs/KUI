# Mission timeline

Track every agent on one shared clock—event peaks since you last looked on top, live Gantt swimlanes with handoffs in the middle, and inline approvals on the right.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add mission-timeline`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::mission_timeline::{
    MissionTimelineBlock, MissionTimelineEvent, TimelineFilter,
};

let timeline = cx.new(|cx| {
    MissionTimelineBlock::new("agent-mission-timeline", cx)
        .with_span_minutes(30)
        .with_default_filter(TimelineFilter::All)
});
cx.subscribe(&timeline, |_, event: &MissionTimelineEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch the unified top filter (`All`, `Running`, `Needs you`, `Failed`) or press Left/Right/Home/End to filter the `"Since you looked"` curve, multi-lane Gantt chart, and `"NEEDS YOU"` cards simultaneously. Hover any colored curve dot to spotlight that agent's swimlane and inbox card, click any Gantt bar to disclose its details, and resolve approvals, choices, or retries inline.

## API

Primary GPUI type: `MissionTimelineBlock` (`kui::components::mission_timeline::MissionTimelineBlock`).
Event type: `MissionTimelineEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation (`894px` down to `390px`).

## Motion

Active `"now"` bars advance gently at the live frontier, cross-lane handoff links animate traveling light pulses (`pro_connector_pulse`), status filter pills glide (`pro_pill_indicator`), and resolving an inbox card updates all three sections in lockstep. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
