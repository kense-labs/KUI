# Morph select

Choose one value from a surface that grows into an option list.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add morph-select`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::morph_select::{MorphOption, MorphSelect};

let options = vec![MorphOption::new("todo", "To do").with_group("Open"), MorphOption::new("done", "Done").with_group("Closed")];
let control = cx.new(|cx| MorphSelect::new("Status", options, window, cx).with_value("todo").with_searchable(true));
```

## Interaction

Open the control, search by label, group, detail or keyword, and use arrows or Page Up/Down to move among enabled results. Enter selects; Escape closes.

## API

`with_searchable` · `(bool) -> Self`

Supply stable values and labels, optional group, detail, keywords and disabled states. with_searchable controls the search field; MorphSelectionChanged emits the chosen value.

Event: `MorphSelectionChanged`.

## Motion

The surface changes width and height on springs, the active highlight glides, and changed labels roll and fade. Reduced motion applies changes immediately.

Desktop and web use the same GPUI view.
