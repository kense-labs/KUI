# Skeleton

Reserve the shape of content while a view is loading.

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
kui add skeleton
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::skeleton::Skeleton;

let control = cx.new(|cx| Skeleton::new("Loading summary", [0.8, 1.0, 0.6], cx).with_avatar());
```

## Interaction

Provide an accessible loading label and relative line widths. Replace the placeholder when data arrives.

## API

`with_avatar` · `Self → Self`

Provide an accessible loading label and relative line widths. Replace the placeholder when data arrives.

## Motion

Placeholder blocks pulse in sequence every 1.8s, offset by 90ms. They stop after eleven cycles and remain static under reduced motion.

Desktop and web use the same GPUI view.
