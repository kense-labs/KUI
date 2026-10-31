# Combobox

Filter single-choice options by name or keyword without losing the committed value.

`kui add combobox`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::combobox::Combobox;
use crate::ui::primitives::picker::PickerOption;

let control = cx.new(|cx| Combobox::new("model", "Model", vec![
        PickerOption::new("fast", "Fast").with_keywords(["quick"]),
        PickerOption::new("reasoning", "Reasoning"),
    ], window, cx).with_value("fast"));
```

PickerOption supports aliases and disabled rows. Clearing emits SelectionChanged(None).

## Interaction

Type to filter, use arrows to highlight and Enter to choose. Escape discards the query. The caret rotates on open. The list springs to its filtered height.

- Event: `SelectionChanged`.
- Preview: [English](/en/kui/components/combobox/) · [中文](/kui/components/combobox/).
