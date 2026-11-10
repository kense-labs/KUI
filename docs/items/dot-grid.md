# Dot grid

Draw a lens that follows the pointer across a decorative dot field, with click and tap ripples. Place normal GPUI content above the field.

## Installation

Initialize KUI with `kui::init(cx)`. Use `cargo add kense-kui --rename kui --no-default-features` or run `kui init` and `kui add dot-grid`. With the CLI, change the import prefix to `crate::ui::`.

## Usage

```rust
use gpui::{div, prelude::*, px};
use kui::components::dot_grid::{DotGrid, DotGridOptions};

let control = cx.new(|cx| {
    DotGrid::new(DotGridOptions::default(), cx)
        .with_style(|root| root.h(px(480.)).flex().items_center().justify_center())
        .with_content(|_, _| div().child("Interfaces with a pulse"))
});
```

## API

| Option | Type | Default | Behavior |
| --- | --- | --- | --- |
| `gap` | `f32` | `22.0` | Center spacing in logical pixels, clamped to 4–256. Very large surfaces can increase spacing to bound work. |
| `dot_size` | `f32` | `1.4` | Resting dot radius in logical pixels, clamped to 0–32. |
| `radius` | `f32` | `150.0` | Pointer lens radius in logical pixels, clamped to 0–1024. |
| `strength` | `f32` | `16.0` | Peak lens displacement in logical pixels, clamped to 0–128. |
| `ripple` | `bool` | `true` | Clicks and taps launch expanding ripples. |
| `fade` | `bool` | `true` | Reduces dot opacity toward the edges. |
| `color` | `Option<Hsla>` | Strong border token | Resting dot color. |
| `active_color` | `Option<Hsla>` | Accent token | Lens and ripple color. |

Use `with_content` for an overlay factory, `with_style` for sizing and chrome, `with_id` for stable identity, and `set_options` for runtime configuration. A host that tracks embedded visibility can provide it through `with_visibility`.

## Interaction and performance

The decorative layer stays out of the keyboard and accessibility order. Overlay controls keep their own input handlers. A mouse click creates one ripple and moving away lets the dots settle. A contact on the exposed field moves the lens until release or cancellation. A tap on overlaid content keeps the content's tap behavior and adds a ripple. Touch dragging on the exposed field captures the direct contact.

The field centers on resize. Ripples propagate at about 620 logical pixels per second, fade over 1.5 seconds and can overlap up to six at once. Dots settle on damped springs. Hidden and settled fields stop requesting frames. Reduced motion keeps the lens tint, holds every dot at its resting position and disables ripples. Desktop and Web use the same GPUI rendering and input state.
