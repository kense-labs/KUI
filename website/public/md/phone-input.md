# PhoneInput

Edit a telephone number with a separate country dialing code.

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
kui add phone-input
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::phone_input::PhoneInput;

let control = cx.new(|cx| PhoneInput::new("Mobile number", "US", window, cx));
```

## Interaction

Search calling codes or country names. Use international digits and possibility separately from verification.

## API

`value` · `(&App) -> PhoneValue`

Search calling codes or country names. Use international digits and possibility separately from verification.

Event: `PhoneValue`.

## Motion

Measured height and animated width drive reveal. Roving selection glides down the list, dialing-code digits roll.

Desktop and web use the same GPUI view.
