# Password strength

Check a new password against a visible policy as you type.

`kui add password-strength`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::password_strength::PasswordStrength;

let control = cx.new(|cx| PasswordStrength::new("password", "New password", window, cx));
```

Strength gives the 0–4 level and each rule result. Supply custom rules with with_policy.

## Interaction

Type or paste, then activate the eye, Suggest one or Save. A failed policy check keeps your entry. Four segments fill in sequence and fulfilled rules draw checks. New errors shake once.

- Event: `StrengthEvent`.
- Preview: [English](/en/kui/components/password-strength/) · [中文](/kui/components/password-strength/).
