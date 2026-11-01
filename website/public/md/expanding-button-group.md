# Expanding button group

Reveal a label in place. Mail, document and publishing actions have separate strips.

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
kui add expanding-button-group
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::expanding_button_group::ExpandingButtonGroup;
use kui::primitives::glyph::Glyph;
use kui::primitives::strip::{Segment};

let control = cx.new(|cx| ExpandingButtonGroup::new("mail", vec![Segment::new("reply", "Reply", Some(Glyph::Undo), 96.), Segment::new("forward", "Forward", Some(Glyph::Redo), 106.)], cx));
```

Mount the entity as a GPUI child. Hover or focus an icon to reveal its label. The neighbors slide; with reduced motion widths update immediately.

## Interface

The control, its nested actions and their highlights use the theme’s `radius.md` (8px by default).

- Event: `Activated(SharedString)`.
- API: `Segment::width`.
- Implementation: `registry/src/components/expanding_button_group.rs`.
- Preview: [English](/en/kui/components/expanding-button-group/) · [中文](/kui/components/expanding-button-group/).
