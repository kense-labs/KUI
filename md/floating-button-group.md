# Floating button group

Spaced controls in a tray; examples include a vertical toolbar and a compact message toolbar.

`kui add floating-button-group`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::floating_button_group::FloatingButtonGroup;
use crate::ui::primitives::glyph::Glyph;
use crate::ui::primitives::strip::{Segment};

let control = cx.new(|cx| FloatingButtonGroup::new("canvas", vec![Segment::new("zoom", "Zoom in", Some(Glyph::ZoomIn), 36.).icon_only(), Segment::new("lock", "Lock", Some(Glyph::Lock), 36.).icon_only().divider()], cx).vertical());
```

Mount the entity as a GPUI child. Arrows skip disabled tools. Undo initially enables Redo. The highlight travels without moving the controls.

## Interface

The control, its nested actions and their highlights use the theme’s `radius.md` (8px by default).

- Event: `Activated(SharedString)`.
- API: `vertical() / set_disabled(id, disabled, cx)`.
- Implementation: `registry/src/components/floating_button_group.rs`.
- Preview: [English](/en/kui/components/floating-button-group/) · [中文](/kui/components/floating-button-group/).
