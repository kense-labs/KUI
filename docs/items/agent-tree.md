# Agent tree

Trace who started whom across an autonomous orchestrator run—showing live status, elapsed time, own spend, and rolled-up branch costs on every card, with branches that fold cleanly to a single summary line.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-tree`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_tree::{AgentTree, AgentTreeEvent};

let tree = cx.new(|cx| AgentTree::new("agent-tree", cx));
cx.subscribe(&tree, |_, event: &AgentTreeEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click the descendant count chevron (`3 ▾` / `2 ▾`) on any branch card to fold its sub-agents into a compact one-line summary (`2 sub-agents folded · $0.20`), click `+ Step spend` to watch `pro_rolling_number` roll costs upward through ancestor branches, or click the `1 needs you` header pill or any card to inspect and approve pending gates.

## API

Primary GPUI type: `AgentTree` (`kui::components::agent_tree::AgentTree`).
Event type: `AgentTreeEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_root(...)`, `with_default_collapsed(...)`, `toggle_branch(...)`, `resolve_agent(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Sub-agent branches fold and unfold smoothly via `pro_fold` while measured `pro_connector_pulse` links follow card bounds frame-by-frame, and branch costs interpolate upward with `pro_rolling_number`. Respects `Theme::reduced_motion` and adapts to narrow viewports by indenting levels vertically.

Desktop and web use the same GPUI source.
