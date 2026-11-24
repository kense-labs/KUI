# Agent graph

Visualize autonomous agents, external tools, and human operators as a living orbital network—where call pulses travel out, answer rings return, high-traffic links thicken, working agents breathe softly, and agents needing human attention turn amber.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-graph`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_graph::{AgentGraph, AgentGraphEvent};

let graph = cx.new(|cx| AgentGraph::new("agent-graph", cx));
cx.subscribe(&graph, |_, event: &AgentGraphEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover any agent or tool disc to highlight its immediate neighborhood, click `Send burst` to watch high-traffic circuit links thicken dynamically, or click the amber `Writer` node (or `1 needs you` header chip) to inspect its call log and `Approve` or `Decline` its pending request.

## API

Primary GPUI type: `AgentGraph` (`kui::components::agent_graph::AgentGraph`).
Event type: `AgentGraphEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_nodes(...)`, `with_edges(...)`, `with_calls(...)`, `with_auto_traffic(...)`, `with_height(...)`, `record_call(...)`, `approve_node(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Outbound calls travel as solid blue comets while answers return as hollow green rings and trigger landing ripples on destination nodes. Link stroke widths scale smoothly with recent call heat (`1.35px` to `3.8px`), working agents breathe softly, and `Theme::reduced_motion` disables continuous motion cleanly.

Desktop and web use the same GPUI source.
