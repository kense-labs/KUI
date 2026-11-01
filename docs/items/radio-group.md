# RadioGroup

Choose exactly one alternative in a labeled list.

## Install

```bash
kui add radio-group
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::radio_group::{RadioGroup, FormOption};

let control = cx.new(|cx| RadioGroup::new("Topics", vec![FormOption::new("motion", "Motion"), FormOption::new("color", "Color")], window, cx).with_selected(&["motion"]));
```

## Interaction

Use one labelled choice for mutually exclusive options.

## API

`selected` · `() -> Vec<SharedString>`

Use one labelled choice for mutually exclusive options.

Event: `SelectionChanged`.

## Motion

One plate follows continuous row position. Old/new dots have independent springs.

Desktop and web use the same GPUI view.
