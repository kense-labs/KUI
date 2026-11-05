# Ghost text

Host-driven inline completion in an editable text field.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add ghost-text`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ghost_text::GhostText;

let control = cx.new(|cx| GhostText::new("Description", window, cx));
```

## Interaction

The host owns suggestion requests. Abort on Cancel and return chunks with their Request ID; old or composing responses are ignored. The example is simulated.

## API

`push_chunk` · `(u64, impl AsRef<str>, bool, &mut Context<Self>)`

Tab accepts all; Cmd/Ctrl/Alt+Right accepts a word; Escape dismisses. The action chip accepts on touch. Selection and IME composition suppress suggestions.

Event: `GhostTextEvent`.

## Motion

The current implementation places text without a fade; polished transitions and native IME/touch testing remain pending.

Desktop and web use the same GPUI view.
