# TagInput

Turn a draft into removable tokens as you type.

## Install

```bash
kui add tag-input
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::tag_input::TagInput;

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
