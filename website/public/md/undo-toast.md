# Undo toast

Remove rows in a reversible batch and confirm the result with the host.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add undo-toast`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::undo_toast::{UndoItem, UndoToast};

let items = vec![UndoItem::new("one", "Design review"), UndoItem::new("two", "Project notes")];
let control = cx.new(|cx| UndoToast::new("Inbox", items, cx));
```

## Interaction

Archive or delete items, then undo before the countdown expires. Repeated removals join the same batch. Hovering or focusing the notice pauses the timer.

The timestamp shares a fixed-width cell with the archive and delete actions. The cell reveals actions on hover, and shows them for keyboard or touch input.

## API

`resolve_commit` · `(u64, bool, &mut Context<Self>)`

CommitRequested contains a batch ID and entries. The host must call resolve_commit with the matching ID; a failure restores rows in their original order. Pending operations stay local until the host commits.

`with_avatars` associates optional image sources with item keys.

Event: `UndoToastEvent`.

## Motion

Rows collapse and reopen on a spring. Undo shows a brief confirmation. Reduced motion applies layout changes immediately.

Desktop and web use the same GPUI view.
