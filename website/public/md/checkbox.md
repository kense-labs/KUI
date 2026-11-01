# Checkbox

Mark one option as checked, unchecked or partially selected.

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
kui add checkbox
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::checkbox::{Checkbox, CheckValue};

let control = cx.new(|_| Checkbox::new("Archive", CheckValue::Off));
```

## Interaction

Mixed activates to On. A disabled box is inert.

## API

`set_value` · `(CheckValue, &mut Context<Self>)`

Mixed activates to On. A disabled box is inert.

Event: `CheckValue`.

## Motion

The mark and fill retain spring position on reversal.

Desktop and web use the same GPUI view.
