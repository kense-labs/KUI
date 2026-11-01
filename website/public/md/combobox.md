# Combobox

Filter single-choice options by name or keyword without losing the committed value.

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
kui add combobox
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::combobox::Combobox;
use kui::primitives::picker::PickerOption;

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
