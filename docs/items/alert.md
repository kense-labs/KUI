# Alert

Keep a persistent status message and its recovery action together.

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
kui add alert
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::alert::{Alert, Severity};

let control = cx.new(|_| Alert::new("Sync paused", "Reconnect to upload changes.", Severity::Warning).with_action("Reconnect"));
```

## Interaction

Offer one recovery action for a persistent status.

## API

`replace` · `Alert`

Offer one recovery action for a persistent status.

Event: `AlertAction`.

## Motion

Changing severity reveals the incoming symbol while the previous one recedes. Heading and detail rise and fade independently.

Desktop and web use the same GPUI view.
