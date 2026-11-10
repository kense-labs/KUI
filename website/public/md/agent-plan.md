# TaskPlan

Review an ordered execution plan and change the sequence of its steps.

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
kui add agent-plan
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_plan::{TaskPlan};

let control = cx.new(|cx| TaskPlan::new("plan", "Review release", vec!["Check changes".into(),"Run tests".into()], window, cx));
```

## Interaction

Rename, drag, add or remove steps in draft. Alt plus the arrow keys reorder a step. Approve to run; pause, resume, retry or skip a failed step.


## API

`new` · `(id, goal, Vec<String>, window, cx)`

TaskPlan takes a goal and ordered step titles. Set model for the named model mark. Listen for PlanEvent and update completed; fail_current exposes recovery for the current step.

Event: `PlanEvent`.


## Motion

Completion moves the progress line over 260ms and draws a check over 220ms. Running uses a spinner and a changing title tone; reduced motion leaves both still.

Desktop and web use the same GPUI view.
