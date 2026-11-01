# RichTextEditor

Compose formatted text and apply toolbar commands to a selection.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add rich-text-editor
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::rich_text_editor::RichTextEditor;

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
