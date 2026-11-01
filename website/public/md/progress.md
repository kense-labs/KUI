# Progress

Display measured completion or an indeterminate operation.

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
kui add progress
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::progress::Progress;

let control = cx.new(|cx| Progress::new("Preparing workspace", Some(0.68), cx));
```

## Interaction

Pass a fraction from zero to one, or None for indeterminate work.

## API

`set_value` · `Option<f32>`

Pass a fraction from zero to one, or None for indeterminate work.

## Motion

The label follows the same spring as the fill. The completion color and check follow the advance.

Desktop and web use the same GPUI view.
