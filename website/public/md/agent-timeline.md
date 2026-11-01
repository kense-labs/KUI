# RunLanes

Follow the steps of an agent run and inspect individual results.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add agent-timeline
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_timeline::{RunLanes, Span};

let control = cx.new(|_| RunLanes::new("run","Release review",vec!["Planner".into()],vec![Span {lane:0,start:0.,end:4.,detail:"Read changes".into(),kind:0,parent:None}]));
```

## Interaction

Place work on a shared time axis. Selecting a span highlights its direct handoff.

## API

`new` · `(id, title, lanes, Vec<Span>)`

Each Span specifies lane, start/end seconds, operation kind and an optional parent index. SpanSelected carries the activated index.

Event: `SpanSelected`.

## Motion

Bars and the playhead share the elapsed time. Each dependency elbow draws into the next lane during the first 360ms of its destination span.

Desktop and web use the same GPUI view.
