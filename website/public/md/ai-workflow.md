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

The host executes steps and resolves matching request IDs. Stop emits Cancel and rejects late responses; Retry resumes the failed step. Branch results must reference a supplied path. The example is simulated and sends nothing.

## API

`resolve` · `(u64, Result<WorkflowResult, SharedString>, &mut Context<Self>)`

Subscribe to RunStep (which contains Box<WorkflowStep>) and Cancel; call resolve with an output, duration and branch path when applicable. Changed reports editor updates. Provide stable unique step and path IDs.

Event: `WorkflowEvent`.

## Motion

A 200 ms connector fill and marker indicate the active step; reduced motion shows the end state and hidden previews do not request animation frames. Path crossfades, measured-height expansion and searchable command menu remain unimplemented. Desktop, Web, screen-reader and visual acceptance are pending.

Desktop and web use the same GPUI view.
