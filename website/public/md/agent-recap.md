# Agent recap

Catch up on what background agents accomplished while you were away—a miniature `PathBuilder` event-peak curve on top paired with equal-height `Needs you`, `Failed`, and `Done` triage lanes below.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-recap`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_recap::{AgentRecap, AgentRecapEvent};

let recap = cx.new(|cx| {
    AgentRecap::new("morning-recap", cx)
        .with_window("6:40 PM", "8:10 AM", "13h 30m")
});
cx.subscribe(&recap, |_, ev: &AgentRecapEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hover or click any peak dot along the overnight curve to draw a crosshair cursor, inspect the event tooltip, and highlight its card below—or hover any card to spotlight its peak dot on the chart. Click `Approve`, `Merge`, `Retry`, `Review`, `Undo`, or `Redo` directly on the fixed-height `90px` cards without shifting neighboring lanes.

## API

Primary GPUI type: `AgentRecap` (`kui::components::agent_recap::AgentRecap`).
Event type: `AgentRecapEvent`.

Supports `RecapKind` (`Needs`, `Failed`, `Done`), `with_window`, `with_events`, `with_limit`, `with_replay_ms`, light/dark `Theme`, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

On mount, the `PathBuilder` Gaussian peak curve sweeps smoothly left to right as peak dots pop in; hovering synchronizes the chart crosshair and card highlight while resolving an item updates counts and badges in place. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
