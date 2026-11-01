# PreviewCard

Inspect an identity or link without leaving the surrounding content.

## Install

```bash
kui add hover-card
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::hover_card::PreviewCard;

let control = cx.new(|cx| PreviewCard::new("Jasmine Brooks","JB",Box::new(|_, _| gpui::div().child("Product designer").into_any_element()),cx));
```

## Interaction

Hover, focus or tap the identity. The panel stays available when the pointer crosses onto it.

## API

`new` · `(name, badge, body, &mut App) → Self`

Pass the identity and a body factory that can render rich content.

Event: `—`.

## Motion

Pointer intent opens after 500ms and closes after 140ms. The edge offset and opacity are independent; reduced motion uses a short fade.

Desktop and web use the same GPUI view.
