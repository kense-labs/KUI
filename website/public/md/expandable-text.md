# ExpandableText

Reveal the rest of a long text and collapse it again.

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
kui add expandable-text
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::expandable_text::ExpandableText;

let control = cx.new(|_| ExpandableText::new("A long release note that wraps across several lines in a narrow column.", 2));
```

## Interaction

Read more appears only when measured text exceeds the line clamp.

## API

`new` · `(text, lines: usize) → Self`

The surrounding layout supplies the width; resizing remeasures the text.

Event: `TextExpanded`.

## Motion

Measured overflow drives opening over 500–600ms and closing over 400–450ms, with a fading edge mask and rotating chevron.

Desktop and web use the same GPUI view.
