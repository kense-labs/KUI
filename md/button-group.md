# Button group

Related commands share a continuous surface. Try the edit, zoom and save/preview rows.

`kui add button-group`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::button_group::ButtonGroup;
use crate::ui::primitives::glyph::Glyph;
use crate::ui::primitives::strip::{Segment};

let control = cx.new(|cx| ButtonGroup::new("editor", vec![Segment::new("save", "Save", Some(Glyph::Save), 93.), Segment::new("preview", "Preview", Some(Glyph::Eye), 104.)], cx));
```

Mount the entity as a GPUI child. Each segment emits its own stable id. Zoom +/- changes the percentage; the middle segment resets it.

The rows are 44px high (36px for zoom) including their shared border. The highlight travels between adjacent segments.

## Interface

The control, its nested actions and their highlights use the theme’s `radius.md` (8px by default).

- Event: `Activated(SharedString)`.
- API: `update_label(id, label, cx)`.
- Implementation: `registry/src/components/button_group.rs`.
- Preview: [English](/en/kui/components/button-group/) · [中文](/kui/components/button-group/).
