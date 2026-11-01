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

Rename, add and reorder steps in draft. Approval locks the plan for execution.

## API

`new` · `(id, goal, Vec<String>, window, cx)`

TaskPlan takes a goal and ordered step titles. Subscribe to PlanEvent, then set completed as work advances.

Event: `PlanEvent`.

## Motion

Completion advances the overall progress spring. Each completed disc tints in while its two-stroke check draws over 220ms.

Desktop and web use the same GPUI view.
