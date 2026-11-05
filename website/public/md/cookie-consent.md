# Cookie consent

Let visitors choose optional cookie categories.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add cookie-consent`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::cookie_consent::{ConsentCategory, CookieConsent};

let categories = vec![ConsentCategory::new("essential", "Essential").with_required(true), ConsentCategory::new("analytics", "Analytics")];
let control = cx.new(|cx| CookieConsent::new(categories, window, cx));
```

## Interaction

Accept all, reject optional categories, or customize choices. Escape or Close discards unsaved changes. Essential categories remain enabled.

## API

`restore` · `(&[ConsentChoice], &mut Context<Self>)`

Provide category IDs and labels. ConsentChanged reports choices; restore loads a previously saved decision. The host persists choices and applies them to integrations.

Event: `ConsentChanged`.

## Motion

The surface grows for preferences and folds to a reopen control after saving; reduced motion applies changes immediately.

Desktop and web use the same GPUI view.
