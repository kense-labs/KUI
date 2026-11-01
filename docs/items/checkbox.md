# Checkbox

Mark one option as checked, unchecked or partially selected.

## Install

```bash
kui add checkbox
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::checkbox::{Checkbox, CheckValue};

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
