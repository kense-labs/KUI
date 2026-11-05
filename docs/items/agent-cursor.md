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

The host supplies document blocks, target positions, run results and persistence. The preview simulates edits and saves nothing.

## API

`set_target_for` · `(u64, impl Into<SharedString>, &mut Context<Self>) -> bool`

Handle Run using its ticket with set_target_for, set_position_for, set_status_for and set_blocks_for; finish or fail the same ticket. Stop invalidates it and restores the original. Review exposes Accept and Undo events. set_position uses local logical pixels; set_visible controls off-screen work.

Event: `AgentCursorEvent`.

## Motion

The cursor follows measured block targets with a spring; reduced motion jumps to the target and off-screen animation stops. Word-by-word streaming, selection effects and full reference comparison remain unimplemented or unverified.

Desktop and web use the same GPUI view.
