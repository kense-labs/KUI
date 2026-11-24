# Task board

Organize engineering sprint work across Kanban columns with live search, label filter chips, a capsule compact density switch, drag-and-drop card moves, inline quick-add forms, and a slide-over task detail drawer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add task-board`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::task_board::{TaskBoardBlock, TaskBoardEvent};

let block = cx.new(|cx| TaskBoardBlock::new("task-board", cx));
cx.subscribe(&block, |_, event: &TaskBoardEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Drag any task card across Backlog, Todo, In Progress, and Done (or use the inline arrow buttons and status radio buttons inside the detail drawer). Click `+` at the bottom of any column to add a task inline, toggle label chips or search to filter, flip the Compact switch to condense cards, and press Escape to close the detail drawer.

## API

Primary GPUI type: `TaskBoardBlock` (`kui::components::task_board::TaskBoardBlock`).
Event type: `TaskBoardEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile horizontal board scrolling.

## Motion

Drop indicator lines highlight landing positions during pointer drag, inline quick-add cards fold smoothly, and the task detail drawer slides in with presence physics while respecting Theme::reduced_motion.

Desktop and web use the same GPUI source.
