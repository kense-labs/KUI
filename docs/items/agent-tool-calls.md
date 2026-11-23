# Agent tool calls

Pair an interactive agent-to-tool topology graph with a resizable call log, latency and failure filters, and JSON payload inspector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-tool-calls`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_tool_calls::{AgentToolCallsScreen, AgentToolCallsEvent};

let screen = cx.new(|cx| AgentToolCallsScreen::new("agent-tool-calls", cx));
cx.subscribe(&screen, |_, event: &AgentToolCallsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch Org, Compact, or pan/zoom Canvas topology, press / to filter the 15-call log, click any node or call to inspect, approve, retry, or copy JSON.

## API

Primary GPUI type: `AgentToolCallsScreen` (`kui::components::agent_tool_calls::AgentToolCallsScreen`).
Event type: `AgentToolCallsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
