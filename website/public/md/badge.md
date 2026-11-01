# Badge

Mark a status or category with a compact text label.

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
kui add badge
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::badge::{Badge, BadgeTone};

let control = cx.new(|_| Badge::new("In review", BadgeTone::Info));
```

## Interaction

Use a short label and tone to identify a status without relying solely on hue.

## API

`set` · `(label, BadgeTone)`

Use a short label and tone to identify a status without relying solely on hue.

## Motion

Changed text crosses over in place with a short directional rise. The foreground, fill and border transition between severity colors.

Desktop and web use the same GPUI view.
