# Floating button group

Spaced controls in a tray; examples include a vertical toolbar and a compact message toolbar.

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
kui add floating-button-group
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::floating_button_group::FloatingButtonGroup;
use kui::primitives::glyph::Glyph;
use kui::primitives::strip::{Segment};

let control = cx.new(|cx| FloatingButtonGroup::new("canvas", vec![Segment::new("zoom", "Zoom in", Some(Glyph::ZoomIn), 36.).icon_only(), Segment::new("lock", "Lock", Some(Glyph::Lock), 36.).icon_only().divider()], cx).vertical());
```

Mount the entity as a GPUI child. Arrows skip disabled tools. Undo initially enables Redo. The highlight travels without moving the controls.

## Interface

The control, its nested actions and their highlights use the theme’s `radius.md` (8px by default).

- Event: `Activated(SharedString)`.
- API: `vertical() / set_disabled(id, disabled, cx)`.
- Implementation: `registry/src/components/floating_button_group.rs`.
- Preview: [English](/en/kui/components/floating-button-group/) · [中文](/kui/components/floating-button-group/).
