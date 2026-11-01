# ExpandableText

Reveal the rest of a long text and collapse it again.

## Install

```bash
kui add expandable-text
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::expandable_text::ExpandableText;

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
