# Agent run detail

Inspect a single autonomous run across 16 waterfall spans with nested sub-agents, token breakdown, reasoning trace, and JSON payloads.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-run-detail`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_run_detail::{AgentRunDetailScreen, AgentRunDetailEvent};

let screen = cx.new(|cx| AgentRunDetailScreen::new("agent-run-detail", cx));
cx.subscribe(&screen, |_, event: &AgentRunDetailEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K or Up/Down steps through spans, Home/End jumps to start or end, Enter opens the right step inspector, and Stop or Rerun controls execution.

## API

Primary GPUI type: `AgentRunDetailScreen` (`kui::components::agent_run_detail::AgentRunDetailScreen`).
Event type: `AgentRunDetailEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
