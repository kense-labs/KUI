# Button group

Related commands share a continuous surface. Try the edit, zoom and save/preview rows.

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
kui add button-group
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::button_group::ButtonGroup;
use kui::primitives::glyph::Glyph;
use kui::primitives::strip::{Segment};

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
