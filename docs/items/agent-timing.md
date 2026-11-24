# Agent timing

Render an autonomous multi-agent run as a hardware-logic-style timing diagram—one rounded step-waveform per participant (`Idle` baseline, `Thinking` low step, `Tool call` high step, `Waiting on you` dashed amber step) with gradient fills, pulsing `"now"` dots, and vertical dashed handoffs.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-timing`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_timing::{
    AgentTiming, AgentTimingEvent, TimingHandoff, TimingParticipant, TimingSegment, TimingState,
};

let timing = cx.new(|cx| {
    AgentTiming::new("cart-timing", cx)
        .with_title("Fix the cart total")
        .with_participants(vec![
            TimingParticipant::person("you", "You"),
            TimingParticipant::agent("lead", "Lead"),
            TimingParticipant::agent("coder", "Coder"),
        ])
        .with_segments(vec![
            TimingSegment::new("lead", TimingState::Thinking, 0.5).with_end(4.0).with_label("Planning the fix"),
            TimingSegment::new("lead", TimingState::Tool, 4.0).with_end(8.5).with_label("search_code"),
            TimingSegment::new("coder", TimingState::Tool, 9.0).with_label("edit_file"),
        ])
        .with_handoffs(vec![
            TimingHandoff::new("you", "lead", 0.4).with_label("Fix the cart total bug"),
            TimingHandoff::new("lead", "coder", 8.8).with_label("Fix rounding in cart.ts"),
        ])
});
cx.subscribe(&timing, |_, ev: &AgentTimingEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hover or click any waveform step or vertical handoff wire (or focus the plot and navigate with `ArrowLeft` / `ArrowRight` / `ArrowUp` / `ArrowDown` / `Home` / `End`, `Escape` to dismiss) to inspect who worked, what tool or thought ran, and exact start/duration timing. Click the refresh icon in the header bar to replay the run from `0:00`.

## API

Primary GPUI type: `AgentTiming` (`kui::components::agent_timing::AgentTiming`).
Supporting types: `TimingParticipant`, `TimingParticipantKind`, `TimingSegment`, `TimingState`, `TimingHandoff`, `TimingInspection`.
Event type: `AgentTimingEvent` (`Inspect(Option<TimingInspection>)`, `ReplayStarted`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeletons, and `with_empty(true)` empty states.

## Motion

`PathBuilder` draws rounded cubic step transitions with soft vertical gradient fills underneath, running steps grow smoothly to the `"now"` cursor with pulsing endpoint halos, active agent 3x3 pixel marks twinkle, and vertical handoff wires draw in as `"now"` crosses them; `Theme::reduced_motion` disables pulses and twinkle cleanly.

Desktop and web use the same GPUI source.
