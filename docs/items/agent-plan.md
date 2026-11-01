# TaskPlan

Review an ordered execution plan and change the sequence of its steps.

## Install

```bash
kui add agent-plan
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::agent_plan::{TaskPlan};

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
