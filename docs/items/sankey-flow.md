# Sankey flow

Follow quantities through stages that split and merge.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add sankey-flow`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::sankey_flow::{FlowLink, FlowNode, SankeyFlow};

let nodes = vec![FlowNode::new("a", "Source", gpui::rgb(0x6a9fe5).into()), FlowNode::new("b", "Activated", gpui::rgb(0xa8acb4).into())];
let links = vec![FlowLink::new("a", "b", 42.0)];
let control = cx.new(|_| SankeyFlow::new("Journey", nodes, links));
```

## Interaction

Use unique node IDs and forward links. Non-finite, non-positive, missing-node and reverse links are ignored. Set visibility from the host to stop particle frames off screen.

## API

`set_data` · `(Vec<FlowNode>, Vec<FlowLink>, &mut Context<Self>)`

Hover, tap or use arrow keys to inspect; period selections emit FlowPeriodChange for the host to update data. Touch and screen-reader behavior still need real-device verification.

Event: `FlowInspect, FlowPeriodChange`.

## Motion

Proportional ribbons and nodes transition between datasets, with optional particles; reduced motion removes particles. Frame-by-frame visual comparison is pending.

Desktop and web use the same GPUI view.
