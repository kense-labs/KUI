# TextMorph

Transition a short changing label while matching repeated characters by occurrence.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add text-morph`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::text_morph::TextMorph;

let control = cx.new(|_| TextMorph::new("Make it simple"));
```

## Interaction

Call set_text with the next label; the width follows measured text.

## API

`set_text` · `(impl Into<SharedString>, &mut Context<Self>)`

Call set_text with the next label; the width follows measured text.

Event: `—`.

## Motion

Matched glyph positions and width use interruptible springs; reduced motion swaps immediately.

Desktop and web use the same GPUI view.
