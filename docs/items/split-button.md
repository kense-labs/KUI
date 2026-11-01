# Split button

Publish immediately or open the adjacent menu for three alternatives.

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
kui add split-button
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::split_button::SplitButton;
use kui::primitives::glyph::Glyph;
use kui::primitives::strip::{Segment, Choice};

let control = cx.new(|cx| SplitButton::new("publish-control", vec![
    Segment::new("publish", "Publish", Some(Glyph::Send), 103.),
    Segment::new("options", "More actions", Some(Glyph::Down), 38.).icon_only()
        .with_menu(vec![Choice::new("draft", "Save as draft")]),
], cx));
```

Mount the entity as a GPUI child. Down opens the menu. Arrows move between enabled options; Enter selects, Escape returns focus to the arrow.

The arrow stays still as the menu opens. The main segment follows the label width.

## Interface

The control, its nested actions and their highlights use the theme’s `radius.md` (8px by default).

- Event: `Activated(SharedString)`.
- API: `Segment::with_menu(Vec<Choice>)`.
- Implementation: `registry/src/components/split_button.rs`.
- Preview: [English](/en/kui/components/split-button/) · [中文](/kui/components/split-button/).
