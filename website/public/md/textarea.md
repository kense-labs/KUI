# Textarea

Write a multiline project note, select text across lines and drag the bottom corner to resize.

`kui add textarea`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::textarea::Textarea;

let control = cx.new(|cx| Textarea::new("notes", "Project note", window, cx).with_value("Ready for review.", window, cx));
```

Textarea preserves line breaks. set_description can display a live count supplied by the host.

## Interaction

Enter inserts a line. Shift with arrows extends the selection; drag the handle to change height. Helper and error feedback stay below the editable area while resizing.

- Event: `FieldEvent`.
- Preview: [English](/en/kui/components/textarea/) · [中文](/kui/components/textarea/).
