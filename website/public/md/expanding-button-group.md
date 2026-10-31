# Expanding button group

Reveal a label in place. Mail, document and publishing actions have separate strips.

`kui add expanding-button-group`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::expanding_button_group::ExpandingButtonGroup;
use crate::ui::primitives::glyph::Glyph;
use crate::ui::primitives::strip::{Segment};

let control = cx.new(|cx| ExpandingButtonGroup::new("mail", vec![Segment::new("reply", "Reply", Some(Glyph::Undo), 96.), Segment::new("forward", "Forward", Some(Glyph::Redo), 106.)], cx));
```

Mount the entity as a GPUI child. Hover or focus an icon to reveal its label. The neighbors slide; with reduced motion widths update immediately.

## Interface

The control uses the theme’s `radius.md` (8px by default) and its nested actions or highlights use `radius.sm` (4px by default).

- Event: `Activated(SharedString)`.
- API: `Segment::width`.
- Implementation: `registry/src/components/expanding_button_group.rs`.
- Preview: [English](/en/kui/components/expanding-button-group/) · [中文](/kui/components/expanding-button-group/).
