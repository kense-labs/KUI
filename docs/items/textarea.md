# Textarea

Write a multiline project note, select text across lines and drag the bottom corner to resize.

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
kui add textarea
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::textarea::Textarea;

let control = cx.new(|cx| Textarea::new("notes", "Project note", window, cx).with_value("Ready for review.", window, cx));
```

Textarea preserves line breaks. set_description can display a live count supplied by the host.

## Interaction

When using a Pinyin IME, the first `n` appears once as marked text. Choosing `你` replaces that text, and cancelling removes the unfinished syllable. Outside composition, Enter inserts a line. Shift with arrows extends the selection; drag the handle to change height. Helper and error feedback stay below the editable area while resizing.

- Event: `FieldEvent`.
- Preview: [English](/en/kui/components/textarea/) · [中文](/kui/components/textarea/).
