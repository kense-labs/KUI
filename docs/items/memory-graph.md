# Memory graph

Radial agent memory and tool-reach map placing an active agent in the center between provenance-backed memory cards on the left (`file`, `chat`, `email`) and connected tool cards on the right—linked by rounded orthogonal connectors with live light pulses, Gradient Floor washes, switches, Forget, and an Add popover.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add memory-graph`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::memory_graph::{MemoryGraph, MemoryGraphEvent};

let graph = cx.new(|cx| {
    MemoryGraph::new("memory-graph", cx)
        .with_title("What Assistant knows and can reach")
});
cx.subscribe(&graph, |_, event: &MemoryGraphEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click any memory card on the left to inspect its provenance file/chat, recall count, `Use this memory` switch, or `Forget` button. Click any tool card on the right to inspect its permission scope and toggle access off. Click the central agent card to open the `Add` dialog where users can drop/import a document or connect an integration from the catalog. Hovering a memory highlights outer dashed links to related or superseded memories.

## API

Primary GPUI type: `MemoryGraph` (`kui::components::memory_graph::MemoryGraph`).
Event type: `MemoryGraphEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_agent(...)`, `with_memories(...)`, `with_tools(...)`, `with_catalog(...)`, `with_using(...)`, `with_recalling(...)`, `with_live(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Light pulses travel along `pro_connector_pulse` connectors between the central agent and active tools or recalled memories, illuminating newly learned or recalled cards with a rising bottom Gradient Floor wash. Disabled nodes fade to `0.38` opacity with dashed connectors. Respects `Theme::reduced_motion` and stacks cleanly below `640px`.

Desktop and web use the same GPUI source.
