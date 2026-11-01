# Password field

Reveal or mask an access key without losing the editing position.

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
kui add password-field
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::password_field::PasswordField;

let control = cx.new(|cx| PasswordField::new("access", "Access key", window, cx));
```

The initial value is masked. FieldEvent::Visibility reports whether it is revealed.

## Interaction

Tab to the eye and press Enter or Space to reveal or mask the same value. A stroke travels across the eye while the value changes visibility in place.

- Event: `FieldEvent`.
- Preview: [English](/en/kui/components/password-field/) · [中文](/kui/components/password-field/).
