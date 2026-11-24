# Project flow

Render an animated product workflow scene where a client task branches via rounded connectors and light pulses into subtasks, notes, and assigned roles.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add project-flow`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::project_flow::{ProjectFlow, ProjectFlowEvent};

let flow = cx.new(|cx| {
    ProjectFlow::new("feature-project-flow", cx)
        .with_client_name("Unify")
        .with_task_title("Task: Web Design")
});
cx.subscribe(&flow, |_, ev: &ProjectFlowEvent, _| {
    let _ = ev;
})
.detach();
```

## Interaction

Hover or click any workflow card (`Client`, `Task`, `Client Notes`, `Subtasks`, `Project Manager`, `UX Designer`) to highlight its incident branches, or click `Replay` (or press `R` / `Space`) to replay the full staggered build-in and hand drag-and-drop sequence.

## API

Primary GPUI type: `ProjectFlow` (`kui::components::project_flow::ProjectFlow`).
Event type: `ProjectFlowEvent` (`kui::components::project_flow::ProjectFlowEvent`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_client_name(...)`, `with_task_title(...)`, `with_selected_node(...)`, `with_auto_intro(...)`, `with_playing(...)`, `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Drawn on a proportional `2312 × 1644` artboard that scales cleanly to its container width without reflowing. Continuous sky-blue pulses glide along all five `#82BCFF` rounded connectors in steady state, while `replay()` builds cards sequentially and drags the stacked `Subtasks` cards into place with a dashed Bezier guide and soft drop bounce; `Theme::reduced_motion` presents the settled layout immediately.

Desktop and web use the same GPUI source.
