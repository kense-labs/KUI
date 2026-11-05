# TextReveal

Show a sentence as a short sequence of rising words.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add text-reveal`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::text_reveal::TextReveal;

let control = cx.new(|cx| TextReveal::new("A quieter way to work.", cx));
```

## Interaction

Set a delay or replay with new text.

## API

`with_delay` · `(f32) -> Self`

Set a delay or replay with new text.

Event: `—`.

## Motion

Words rise with a capped stagger; reduced motion shows the text immediately.

Desktop and web use the same GPUI view.
