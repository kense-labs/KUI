# Data flow

Illustrate how host-provided sources feed a processing step and destinations.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add data-flow`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::data_flow::{DataFlow, DataFlowNode, DataFlowSnapshot, DataFlowStatus};

let sources = vec![DataFlowNode::new("app", "App")];
let destinations = vec![DataFlowNode::new("store", "Warehouse")];
let snapshot = DataFlowSnapshot::new(42, DataFlowStatus::Running);
let control = cx.new(|_| DataFlow::new(sources, DataFlowNode::new("core", "Transform"), destinations, snapshot));
```

## Interaction

The host supplies the processed count and service state; the illustration never generates real records. Use stable node IDs and destination shares between zero and one.

## API

`with_node_icon` · `(impl Into<SharedString>, Glyph) -> Self`. Associates a KUI glyph with a node ID. Nodes display their text by default.

`set_snapshot` · `(DataFlowSnapshot, &mut Context<Self>)`

Use arrows, Home and End to inspect nodes; set_visible stops off-screen animation and set_paused controls illustration playback. The demo uses explicitly labelled sample data.

Event: `DataFlowInspect, DataFlowPlaybackChanged`.

## Motion

Hollow source markers become solid destination markers, with reduced motion showing a still frame. Detailed path motion, accessibility and device behavior still require verification.

Desktop and web use the same GPUI view.
