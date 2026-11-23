# Kanban

Triage engineering issues across priority columns and list views with pointer drag-and-drop, multi-criteria filters, New task modal, and full issue editor.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add kanban`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::kanban::{KanbanScreen, KanbanEvent};

let screen = cx.new(|cx| KanbanScreen::new("kanban", cx));
cx.subscribe(&screen, |_, event: &KanbanEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Drag cards across columns or reorder with Alt+Arrows, press / to search, Filter/Sort to refine, N to create a task, and Enter to open the issue detail view.

## API

Primary GPUI type: `KanbanScreen` (`kui::components::kanban::KanbanScreen`).
Event type: `KanbanEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
