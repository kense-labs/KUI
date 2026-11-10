# AI workflow

Build and test a short host-driven automation with a branch.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add ai-workflow`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ai_workflow::{AiWorkflow, WorkflowKind, WorkflowPath, WorkflowStep};

let steps = vec![WorkflowStep::new("trigger", "Receive request", "Inbox", WorkflowKind::Trigger)];
let paths: Vec<WorkflowPath> = vec![];
let control = cx.new(|_| AiWorkflow::new("Request triage", "Review incoming requests", steps, paths));
```

## Interaction

The host executes steps and resolves matching request IDs. Stop emits Cancel and rejects late responses; Retry resumes the failed step. Branch results must reference a supplied path. The simulated preview sends nothing and offers Passes, Fails, Loading and Empty scenarios. The empty example lists available triggers; triggers in configured workflows cannot be removed through the step list.

## API

`resolve` · `(u64, Result<WorkflowResult, SharedString>, &mut Context<Self>)`

Subscribe to RunStep (which contains Box<WorkflowStep>) and Cancel; call resolve with an output, duration and branch path when applicable. Changed reports editor updates. Provide stable unique step and path IDs.

Use set_loading(bool, cx) for a loading skeleton. Loading cancels any pending run. The step picker searches title, app, and detail. Use Up or Down to move, Enter to add, and Escape to close.

Event: `WorkflowEvent`.

## Motion

A 240 ms marker traces the incoming connector as a step starts. Completed results expand to their measured height and fade in under their step over 240 ms. Reduced motion shows each state immediately. The branch list switches immediately.

Desktop and web use the same GPUI view.
