# WorkBadge

Show the current activity of an assistant during a run.

## Install

```bash
kui add agent-status
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::agent_status::{WorkBadge, Step};

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
