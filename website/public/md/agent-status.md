# WorkBadge

Show the current activity of an assistant during a run.

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
kui add agent-status
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_status::{WorkBadge, Step};

let control = cx.new(|_| WorkBadge::new("agent","Assistant","Ready",vec![Step { label:"Check files".into(),result:String::new(),complete:false }]));
```

## Interaction

Keep the trigger compact. Display detailed progress only while expanded.

## API

`new` · `(id, agent, activity, Vec<Step>)`

The host updates activity, elapsed seconds and step results. WorkBadge owns the disclosure state.

## Motion

The active label crossfades with 2px travel and its measured single-line width retargets on a 240ms spring. A rotating arc marks unfinished steps; reduced motion leaves the arc still, and completing the run replaces the centre glyph with a check.

Desktop and web use the same GPUI view.
