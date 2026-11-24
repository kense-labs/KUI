# Agent checkpoints

Render an autonomous agent run as a Git-like `#82bcff` checkpoint rail where rewinding to any prior snapshot branches a new parallel attempt without deleting history.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-checkpoints`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_checkpoints::{
    AgentCheckpoints, AgentCheckpointsEvent, Checkpoint, CheckpointStatus,
};

let checkpoints = cx.new(|cx| {
    AgentCheckpoints::new("pricing-run", cx)
        .with_task("Ship the pricing page")
        .with_checkpoints(vec![
            Checkpoint::new("c1", "Read the brief").with_icon("slack").with_at_ms(9_000).with_cost(0.03),
            Checkpoint::new("c2", "Scaffolded the pricing page").with_parent("c1").with_icon("github").with_files(4).with_at_ms(30_000).with_cost(0.06),
            Checkpoint::new("c3", "Built the pricing table").with_parent("c2").with_icon("figma").with_files(3).with_at_ms(81_000).with_cost(0.11).with_status(CheckpointStatus::Running),
        ])
});
cx.subscribe(&checkpoints, |_, ev: &AgentCheckpointsEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hovering or focusing any prior checkpoint reveals `Rewind` in place of the file count and elapsed time so row height never jumps. Activating `Rewind` (by click or `Enter` / `Space` after moving with `ArrowUp` / `ArrowDown` / `Home` / `End`) marks any `Running` head as `Stopped`, dims the abandoned branch, and bends a new parallel attempt lane out of the chosen parent checkpoint.

## API

Primary GPUI type: `AgentCheckpoints` (`kui::components::agent_checkpoints::AgentCheckpoints`).
Supporting types: `Checkpoint`, `CheckpointStatus`, `format_checkpoint_ms`.
Event type: `AgentCheckpointsEvent` (`Rewind { parent_id, new_head_id, attempt }`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton rows, and `with_empty(true)` empty states.

## Motion

`Slide` springs expand the left lane gutter and card height as parallel attempts branch via `PathBuilder` S-curves, newly created checkpoints rise in with a dot glint, and `Running` titles shimmer; `Theme::reduced_motion` renders updates immediately without continuous animation.

Desktop and web use the same GPUI source.
