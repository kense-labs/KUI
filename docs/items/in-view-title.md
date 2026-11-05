# InViewTitle

Reveal a heading when at least 45% is visible.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add in-view-title`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::in_view_title::InViewTitle;

let control = cx.new(|_| InViewTitle::new("Make room for ideas"));
```

## Interaction

Use once for a one-time entrance, or disable it to replay.

## API

`with_once` · `(bool) -> Self`

Use once for a one-time entrance, or disable it to replay.

Event: `—`.

## Motion

The title rises and fades in; reduced motion displays it immediately.

Desktop and web use the same GPUI view.
