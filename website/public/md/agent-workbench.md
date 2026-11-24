# Agent workbench

Spatial single-agent workbench connecting the person who asked on the left, the live agent and its three-row work log in the center, uploaded files under a macOS folder on the right, and connected tool cards below—joined by rounded orthogonal wires with hollow ports and a traveling light pulse.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-workbench`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_workbench::{AgentWorkbench, AgentWorkbenchEvent};

let workbench = cx.new(|cx| {
    AgentWorkbench::new("agent-workbench", cx)
        .with_live(true)
});
cx.subscribe(&workbench, |_, event: &AgentWorkbenchEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click the central agent, any uploaded file row, or any bottom tool card (or focus with Tab and press Enter/Space) to open its detail popover showing permissions, account metadata, and the steps recorded for this request. Press `Escape` or click outside to dismiss. Use `Next step` or `Auto`/`Pause` to inspect each step's active connector pulse.

## API

Primary GPUI type: `AgentWorkbench` (`kui::components::agent_workbench::AgentWorkbench`).
Event type: `AgentWorkbenchEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_user(...)`, `with_agent(...)`, `with_files(...)`, `with_tools(...)`, `with_steps(...)`, `with_reading(...)`, `with_done(...)`, `with_live(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

A glowing light pulse runs along the rounded orthogonal wire of the active step (`user -> agent`, `agent -> files`, or `agent -> tool`), while the agent's 3x3 pixel mark twinkles, the active step shimmers via `pro_shimmer_text`, and detail cards rise in via `pro_presence`. Respects `Theme::reduced_motion` and stacks cleanly into a single column below `720px`.

Desktop and web use the same GPUI source.
