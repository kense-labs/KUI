# Morph loader

Display an operation as a loading shape that transitions into success or failure.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component, use `kui init` and `kui add morph-loader`, then replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::morph_loader::{LoaderStatus, LoaderVariant, MorphLoader};

let loader = cx.new(|_| MorphLoader::new("publish-status")
    .with_size(24.)
    .with_stroke_width(2.5)
    .with_tone(true)
    .with_labels(["Publishing", "Published", "Publish failed"]));
loader.update(cx, |view, cx| view.set_variant(LoaderVariant::Ring, cx));
// Call only after the host operation has completed.
loader.update(cx, |view, cx| view.set_status(LoaderStatus::Success, cx));
```

## API

| Method | Input | Default | Behavior |
| --- | --- | --- | --- |
| `new` | stable `ElementId` | required | Fixed footprint, loading dots. |
| `with_size` | positive finite `f32` | `24.` | Physical size; geometry remains on a 24-unit grid. |
| `with_stroke_width` | positive finite `f32` | `2.5` | Ring, square, check and cross width in grid units. Dots use width 4, bars use width 3. |
| `with_tone` | `bool` | `true` | Completion uses success/danger theme colors. False inherits the parent text color. |
| `with_labels` | three strings | `["Loading", "Done", "Failed"]` | Loading, success and error accessible labels, respectively. |
| `decorative` | `bool` | `false` | True suppresses the status role and label. |
| `set_variant` | `LoaderVariant`, context | `Dots` | Morph into `Dots`, `Bars`, `Ring` or `Square`, preserving the loop phase. |
| `set_status` | `LoaderStatus`, context | `Loading` | Host reports `Loading`, `Success` or `Error`. Does not automatically reset the host operation. |
| `set_visible` | `bool`, context | `true` | Pause while mounted but hidden; resume without accumulating hidden time. |
| `variant`, `status` | none | current value | Read the current controlled state. |

Invalid non-finite or non-positive size/width inputs preserve the preceding value. Builders configure the control before mounting. There is no completion callback: the host owns the operation.

## Motion

Four strokes interpolate center, length, angle, curvature, width and opacity independently. Retargeting retains instantaneous velocity. Ring segments are circular arcs, not chords. Loading dots rise and thicken in a wave; bars pulse in sequence; ring arcs breathe while rotating; square edges shorten as the drawing makes quarter turns.

Shape changes use a 0.46s critical spring; result strokes use a 0.52s critical spring, staggered 35ms. Rotation settles forward to upright with a 0.60s critical spring. These are spring parameters, not a promise to be fully stationary at that time. The result scales `1 → 1.14 → 1` over 420ms. Reduced motion draws a static shape and does not schedule a loop.

Tone transitions take 240ms, or 120ms with reduced motion, and restart from the current visible color when interrupted.

Only a visible loading or settling control requests another frame. A completed stable result is idle. The footprint stays fixed across all states.

## Preview

The story uses a 72px loader centered in a 220px stage; shape choices and result actions sit below it. Result actions are disabled after one activation, and the demo returns to loading after 1700ms. This timer is story data and is absent from the installable component. Arrow keys, Home and End select a shape; Enter and Space activate result actions. Rectangular controls use the KUI radius token.

Desktop and web use the same GPUI implementation.
