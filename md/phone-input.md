# PhoneInput

Edit a telephone number with a separate country dialing code.

## Install

```bash
kui add phone-input
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::phone_input::PhoneInput;

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
