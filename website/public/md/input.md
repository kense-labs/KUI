# Input

Edit a single line with a visible label, helper text and validation.

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
kui add input
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::input::Input;

let control = cx.new(|cx| Input::new("project", "Project name", window, cx).with_description("A short descriptive name."));
```

FieldEvent reports changes as you type and when you submit. set_error accepts an empty string to clear validation.

## Interaction

Tab or click the label to focus. Select, paste or use an IME. Enter submits. The border tracks focus; validation appears beneath the field.

- Event: `FieldEvent`.
- Preview: [English](/en/kui/components/input/) · [中文](/kui/components/input/).
