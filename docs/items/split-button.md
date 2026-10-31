# Split button

Publish immediately or open the adjacent menu for three alternatives.

`kui add split-button`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::split_button::SplitButton;
use crate::ui::primitives::glyph::Glyph;
use crate::ui::primitives::strip::{Segment, Choice};

let control = cx.new(|cx| SplitButton::new("publish-control", vec![
    Segment::new("publish", "Publish", Some(Glyph::Send), 103.),
    Segment::new("options", "More actions", Some(Glyph::Down), 38.).icon_only()
        .with_menu(vec![Choice::new("draft", "Save as draft")]),
], cx));
```

Mount the entity as a GPUI child. Down opens the menu. Arrows move between enabled options; Enter selects, Escape returns focus to the arrow.

The arrow stays still as the menu opens. The main segment follows the label width.

## Interface

The control uses the theme’s `radius.md` (8px by default) and its nested actions or highlights use `radius.sm` (4px by default).

- Event: `Activated(SharedString)`.
- API: `Segment::with_menu(Vec<Choice>)`.
- Implementation: `registry/src/components/split_button.rs`.
- Preview: [English](/en/kui/components/split-button/) · [中文](/kui/components/split-button/).
