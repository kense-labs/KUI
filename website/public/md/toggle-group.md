# Toggle group

Choose one or several compact toolbar options.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add toggle-group`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::toggle_group::{ToggleGroup, ToggleMode, ToggleOption};

let items = vec![ToggleOption::new("left", "Left"), ToggleOption::new("center", "Center")];
let control = cx.new(|cx| ToggleGroup::new("Alignment", items, ToggleMode::Single, cx).with_selected(&["left"]));
```

## Interaction

Single mode selects one item; multiple mode toggles items independently. Arrow keys move through enabled items.

## API

`with_selected` · `(&[&str]) -> Self`

Supply stable keys and labels, optional icons and disabled states; ToggleChanged returns selected keys.

Event: `ToggleChanged`.

## Motion

The selected indicator moves in single mode; multiple fills transition independently.

Desktop and web use the same GPUI view.

## Verification

2026-11-05 web measurement (900×760, light and dark, `?story=toggle-group`):

- Both modes share one page: single-mode Alignment (Left selected) and multiple-mode Formatting (Bold selected). In dark the selected surface (55,55,55) separates from the track (25,25,25).
- Multiple mode fills independently: clicking Italic lights Bold and Italic both at full fill; turning Bold off leaves only Italic filled (Bold keeps only its hover color). The fills are per-segment, not one shared surface.
- Single mode moves the surface with both `x` and `width` on 0.42s critical springs (`toggle_group.rs:59-60`, ω = 12.47): left edge 275→317 and width 46→63 device px over 24 flattened frames, monotonic and decelerating. A wider segment grows smoothly rather than snapping.
- Reduced motion applies the whole change in one frame (275..320 / w46 → 323..387 / w65, no intermediate).
- Keyboard (`→` ×2 after focusing Left) moves the surface to Right with a visible focus ring. 320px Chinese renders both groups unwrapped with no missing glyphs.
- Boundary: at 125% zoom the story panel reflows (effective CSS width changes, the panel shifts to the inline edge), so no ≤1px geometry claim is made at 125%. Segment width follows label text plus padding; contrast was not measured numerically. Not compared frame-by-frame against a same-speed 60Hz baseline.
