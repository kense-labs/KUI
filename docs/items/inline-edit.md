# Inline edit

Rename a project or edit its description in place, with validation and retry.

`kui add inline-edit`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::inline_edit::InlineEdit;

let control = cx.new(|cx| InlineEdit::new("name", "Project name", "KUI project", window, cx).with_validator(|s| s.is_empty().then(|| "Enter a name.".into())));
```

Subscribe to SaveRequest and call finish with its revision and result. An outdated response is ignored.

## Interaction

Activate the value to edit. Enter saves, Escape cancels, leaving saves. Shift+Enter inserts a line in descriptions. The pending draft appears immediately. Success draws a check; failure shows the saved value and a retry.

- Event: `SaveRequest`.
- Preview: [English](/en/kui/components/inline-edit/) · [中文](/kui/components/inline-edit/).
