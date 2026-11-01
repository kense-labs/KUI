# Toast

Acknowledge a short-lived outcome with an optional follow-up action.

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
kui add toast
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::toast::Toast;

let control = cx.new(|_| Toast::new());
```

## Interaction

Call show on the entity. Hover pauses dismissal. Undo and dismiss are distinct events.

## API

`show` · `(message, Option<SharedString>, Duration)`

Call show on the entity. Hover pauses dismissal. Undo and dismiss are distinct events.

Event: `ToastEvent`.

## Motion

The notification rises 16px, draws its check and fades out while dropping 8px. Drag can dismiss it and hover pauses its timeout.

Desktop and web use the same GPUI view.
