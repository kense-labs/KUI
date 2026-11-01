# RichTextEditor

Compose formatted text and apply toolbar commands to a selection.

## Install

```bash
kui add rich-text-editor
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::rich_text_editor::RichTextEditor;

let control = cx.new(|cx| RichTextEditor::new("Release notes", "# Notes", window, cx));
```

## Interaction

Edit formatted blocks directly. Select text for inline marks or links, type / for blocks, and undo edits in the document history.

## API

`document` · `() -> &RichDocument`

Edit formatted blocks directly. Select text for inline marks or links, type / for blocks, and undo edits in the document history.

Event: `RichDocument`.

## Motion

Selection adds/removes the bar through a measured reveal. Both faces paint during transition.

Desktop and web use the same GPUI view.
