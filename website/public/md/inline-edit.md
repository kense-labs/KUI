# Inline edit

Rename a project or edit its description in place, with validation and retry.

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
kui add inline-edit
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::inline_edit::InlineEdit;

let control = cx.new(|cx| InlineEdit::new("name", "Project name", "KUI project", window, cx).with_validator(|s| s.is_empty().then(|| "Enter a name.".into())));
```

Subscribe to SaveRequest and call finish with its revision and result. An outdated response is ignored.

## Interaction

Activate the value to edit. Enter saves, Escape cancels, leaving saves. Shift+Enter inserts a line in descriptions. The pending draft appears immediately. Success draws a check; failure shows the saved value and a retry.

- Event: `SaveRequest`.
- Preview: [English](/en/kui/components/inline-edit/) · [中文](/kui/components/inline-edit/).
