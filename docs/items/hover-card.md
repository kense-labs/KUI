# PreviewCard

Inspect an identity or link without leaving the surrounding content.

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
kui add hover-card
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::hover_card::PreviewCard;

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
