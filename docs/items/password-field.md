# Password field

Reveal or mask an access key without losing the editing position.

`kui add password-field`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::password_field::PasswordField;

let control = cx.new(|cx| PasswordField::new("access", "Access key", window, cx));
```

The initial value is masked. FieldEvent::Visibility reports whether it is revealed.

## Interaction

Tab to the eye and press Enter or Space to reveal or mask the same value. A stroke travels across the eye while the value changes visibility in place.

- Event: `FieldEvent`.
- Preview: [English](/en/kui/components/password-field/) · [中文](/kui/components/password-field/).
