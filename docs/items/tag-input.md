# TagInput

Turn a draft into removable tokens as you type.

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
kui add tag-input
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tag_input::TagInput;

let control = cx.new(|cx| TagInput::new("Tags", window, cx).with_tags(&["release", "web"]));
```

## Interaction

Enter or comma commits unique tags. Empty Backspace arms then removes the last tag.

## API

`tags` · `() -> &[SharedString]`

Enter or comma commits unique tags. Empty Backspace arms then removes the last tag.

Event: `TagsChanged`.

## Motion

When tags are inserted or removed, each remaining tag springs in both axes to its measured slot, including across rows. Interrupted moves continue from the visible position.

Desktop and web use the same GPUI view.
