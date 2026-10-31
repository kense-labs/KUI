# Input

Edit a single line with a visible label, helper text and validation.

`kui add input`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::input::Input;

let control = cx.new(|cx| Input::new("project", "Project name", window, cx).with_description("A short descriptive name."));
```

FieldEvent reports changes as you type and when you submit. set_error accepts an empty string to clear validation.

## Interaction

Tab or click the label to focus. Select, paste or use an IME. Enter submits. The border tracks focus; validation appears beneath the field.

- Event: `FieldEvent`.
- Preview: [English](/en/kui/components/input/) · [中文](/kui/components/input/).
