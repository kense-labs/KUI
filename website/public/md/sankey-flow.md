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
let control = cx.new(|_| {
    SankeyFlow::new("Journey", nodes, links)
        .with_unit(" people")
        .with_total_label("signups")
        .with_columns(vec!["Source".into(), "Activated".into()])
        .with_periods(vec!["Q2".into(), "Q3".into()], 1)
        .with_period_details(vec!["Apr to Jun 2026".into(), "Jul to Sep 2026".into()])
});
```

## Interaction

Use unique node IDs and forward links. Non-finite, non-positive, missing-node and reverse links are ignored. Set visibility from the host to stop particle frames off screen.

In the upright layout, labels align with the start of each node and stay within its allotted width. Narrow labels may show an ellipsis. Inspect the node or read the flow table for the full name and value.

## API

`set_data` · `(Vec<FlowNode>, Vec<FlowLink>, &mut Context<Self>)`

`with_unit` appends a suffix to tooltip and table values. Include any desired leading space. `with_total_label` names the summary total. `with_columns` labels the stages. `with_periods` builds the period buttons and selects one by index. `with_period_details` adds date ranges to the summary.

`with_height(f32)` sets the plot height, which defaults to 340px. Below 420px wide the diagram rotates and uses at least 480px. `with_empty_label` sets the message for empty data. `with_value_formatter(impl Fn(f64) -> String + 'static)` customizes the displayed quantities.

`with_particles(bool)` sets the initial particle state, which defaults to enabled. `with_controlled_particles(bool)` makes the Flow toggle emit a request for the host to apply with `set_particles(bool, &mut Context<Self>)`. `with_particle_toggle(bool)` controls whether the toggle is shown and defaults to `true`.

Tab focuses the first node. Arrow keys navigate the nodes in each stage and select the closest node in the neighboring stage. Home and End go to the first and last nodes. Escape clears the trace. Hover inspects a node or link. On touch, tap to pin it and tap it again to clear it.

Events include `FlowInspect`, `FlowPeriodChange` and `FlowParticlesChange`.

## Motion

Proportional ribbons and nodes transition between datasets on a 0.4s first-reveal spring and a 0.42s data-swap spring. Inspecting a flow fills the traced fraction of each ribbon and node on a 0.26s spring. A source trace uses its series color. A trace starting in a later stage uses a neutral color. Clearing the highlight drains the fill from its current position. Reduced motion applies the trace immediately and hides the particle stream and toggle. Desktop and web use the same GPUI view.
