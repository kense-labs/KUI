# Password strength

Check a new password against a visible policy as you type.

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
kui add password-strength
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::password_strength::PasswordStrength;

let control = cx.new(|cx| PasswordStrength::new("password", "New password", window, cx));
```

Strength gives the 0–4 level and each rule result. Supply custom rules with with_policy.

## Interaction

Type or paste, then activate the eye, Suggest one or Save. A failed policy check keeps your entry. Four segments fill in sequence and fulfilled rules draw checks. New errors shake once.

- Event: `StrengthEvent`.
- Preview: [English](/en/kui/components/password-strength/) · [中文](/kui/components/password-strength/).
