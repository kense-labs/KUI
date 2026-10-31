# Select

Choose one value from a short list.

`kui add select`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::select::Select;
use crate::ui::primitives::picker::PickerOption;

let control = cx.new(|cx| Select::new("team", "Team", vec![
        PickerOption::new("design", "Design"), PickerOption::new("engineering", "Engineering"),
    ], window, cx).with_value("design"));
```

Option values are stable identifiers. Subscribe to SelectionChanged for user selections.

## Interaction

Open with Enter, Space or Down. Arrow keys skip disabled rows. Escape retains the current value. The shown value rolls up or down according to the option order.

- Event: `SelectionChanged`.
- Preview: [English](/en/kui/components/select/) · [中文](/kui/components/select/).
