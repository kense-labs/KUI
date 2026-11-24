# Handoff swimlanes

Map who does what in a multi-agent run—a lane per person, agent, and tool with task boxes, `PathBuilder` approval diamonds, live handoff pulses, and inline approval cards.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add handoff-swimlanes`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::handoff_swimlanes::{HandoffSwimlanes, HandoffSwimlanesEvent};

let swimlanes = cx.new(|cx| {
    HandoffSwimlanes::new("billing-swimlanes", cx)
        .with_title("Ship the billing fix")
});
cx.subscribe(&swimlanes, |_, ev: &HandoffSwimlanesEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Click any lane header (`You`, `Planner`, `Coder`, `Writer`, `Reviewer`, `Vercel`), task box, start/end circle, or approval diamond to inspect its detail card with `From` and `To` handoffs. Waiting approval gates (`Approve deploy`) expose `Reject` and `Approve` actions directly inside the card; pressing `Escape` closes any open card.

## API

Primary GPUI type: `HandoffSwimlanes` (`kui::components::handoff_swimlanes::HandoffSwimlanes`).
Event type: `HandoffSwimlanesEvent`.

Supports `SwimLaneKind` (`Person`, `Agent`, `Tool`), `SwimTaskKind` (`Task`, `Gate`, `Start`, `End`), `SwimTaskStatus` (`Queued`, `Active`, `Waiting`, `Done`, `Failed`, `Skipped`), light/dark `Theme`, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Active handoffs animate traveling light pulses via `pro_connector_pulse`, running tasks sweep a soft sheen while their 3x3 agent pixel mark twinkles, and waiting approval diamonds breathe gently. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
