# SkeletonMorph

A GPUI loading region that transitions measured placeholders into ready content.

```sh
kui add skeleton-morph
```

```rust
use gpui::{div, prelude::*, px};
use kui::components::skeleton_morph::{MorphBlock, SkeletonMorph};

let profile = cx.new(|cx| SkeletonMorph::new(
    "profile-loading",
    true,
    |layout| div().child(layout.block(
        MorphBlock::new("profile-name").with_width(px(120.)).with_lines(1),
        |_, _| div().line_height(px(20.)).child("Riley Morgan").into_any_element(),
    )).into_any_element(),
    cx,
));
```

For copied source, use `crate::ui::components::skeleton_morph`. The host sets the state with `set_loading`. Build the layout in reading order and give each block a unique, stable ID within its region.

| Region option | Rust type | Default |
| --- | --- | --- |
| Loading | `bool` | Required |
| Layout | `Fn(&mut MorphLayout<'_>) -> AnyElement` | Required |
| `with_stagger` | `f32` | 0.04 seconds |
| `with_loading_label` | `Into<SharedString>` | Loading, localized |
| `with_role` | `Role` | Group |
| `with_style` | `Fn(Div) -> Div` | No custom style |
| `with_visibility` | `Fn(&App) -> bool` | Window visibility |

| Block option | Rust type | Default |
| --- | --- | --- |
| Ready content | `FnOnce(&mut Window, &mut App) -> AnyElement` | Required at `layout.block` |
| `with_width` | `Into<DefiniteLength>` | 100% |
| `with_height` | `Into<DefiniteLength>` | 12px, or the text line boxes |
| `with_radius` | `MorphRadius` | Theme radius. Use `Circle` for an avatar |
| `with_lines` | `usize` | Unset. Multiple lines shorten the last bar |
| `with_line_height` | `Pixels` | 20px |
| `with_role` | `Role` | Unset |
| `with_style` | `Fn(Div) -> Div` | No custom style |

Loading shapes paint without mounting the ready content. Each reveal uses the measured starting box and the ready bounds, and the region clips to its eased height. Use explicit line heights and similar placeholder widths for predictable reflow. Reloading immediately restores the skeleton. Reduced motion removes stretching and pulsing and uses a short fade. The loading label is exposed through a status node, and the ready content retains its GPUI semantics.
