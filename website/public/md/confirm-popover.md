# ConfirmPopover

Ask for a deliberate confirmation near a consequential action.

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
kui add confirm-popover
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::confirm_popover::ConfirmPopover;

let control = cx.new(|cx| ConfirmPopover::new("Delete", "Remove this project?", Some("Atlas".into()), window, cx));
```

## Interaction

The 32px confirmation field has 12px of horizontal padding. Clicking the padding focuses the text editor. Enter submits once the full phrase matches and input composition has finished. A failed request can be retried in place.

## API

`resolve` · `(&mut self, ticket: u64, ok: bool, &mut Context<Self>)`

Subscribe to a request ticket and call resolve with that same ticket after the host operation completes.

Event: `ConfirmRequested`.

## Motion

Width and height grow from the trigger while the 8px radius stays fixed. The content keeps its expanded width and fades in after the surface starts opening.

Desktop and web use the same GPUI view.
