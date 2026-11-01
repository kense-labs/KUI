# Select

Choose one value from a short list.

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
kui add select
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::select::Select;
use kui::primitives::picker::PickerOption;

let control = cx.new(|cx| Select::new("team", "Team", vec![
        PickerOption::new("design", "Design"), PickerOption::new("engineering", "Engineering"),
    ], window, cx).with_value("design"));
```

Option values are stable identifiers. Subscribe to SelectionChanged for user selections.

## Interaction

Open with Enter, Space or Down. Arrow keys skip disabled rows. Escape retains the current value. The shown value rolls up or down according to the option order.

- Event: `SelectionChanged`.
- Preview: [English](/en/kui/components/select/) · [中文](/kui/components/select/).
