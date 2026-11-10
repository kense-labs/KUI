# Agent cursor

A host-driven, read-only document and review cursor.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add agent-cursor`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_cursor::{AgentCursor, DocBlock, DocBlockKind};

let blocks = vec![DocBlock::new("body", DocBlockKind::Paragraph, "A short draft.")];
let control = cx.new(|_| AgentCursor::new("Document", blocks));
```

## Interaction

The host supplies document blocks, target positions, run results and persistence. The preview opens with a successful sample, with separate controls for replaying edits or simulating a dropped connection. Retry keeps the original draft and starts a new request.

## API

`set_target_for` · `(u64, impl Into<SharedString>, &mut Context<Self>) -> bool`

Handle `Run` using its ticket with `set_target_for`, `set_position_for`, `set_status_for` and `set_blocks_for`. Call `select_target_for` to highlight a block before streaming its replacement. Finish or fail the same ticket. Stop invalidates it and restores the original. Review exposes Accept and Undo events. `set_position` uses local logical pixels. `with_meta` adds an optional byline, and `is_visible` combines viewport and host visibility for scheduling playback.

`with_collaborator(name, color)` enables an optional second cursor. Selection fills progressively using UTF-8 character boundaries.

Event: `AgentCursorEvent`.

## Motion

The cursor follows measured text positions with a spring. The sample selects a paragraph, adds replacement text in chunks, inserts a heading and fixes a typo. Reduced motion applies the finished edit together. Playback waits while the preview is off screen.

Desktop and web use the same GPUI view.
