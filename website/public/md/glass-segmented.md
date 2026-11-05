# Glass segmented

Switch a view on a translucent segmented surface.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add glass-segmented`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::glass_segmented::{GlassSegmentOption, GlassSegmented};

let options = vec![GlassSegmentOption::new("overview", "Overview"), GlassSegmentOption::new("details", "Details")];
let control = cx.new(|cx| GlassSegmented::new("View", options, cx).with_value("overview"));
```

## Interaction

Use arrows, Home and End to select; drag the selected segment or fling it to settle on another enabled item.

## API

`set_value` · `(&str, &mut Context<Self>)`

Provide stable option values and optional icons or badges. Subscribe to GlassSegmentChanged to update the host. The surface is translucent; true background blur is not currently available.

Event: `GlassSegmentChanged`.

## Motion

Leading and trailing edges move on independent springs and narrow while stretched. Reduced motion tracks directly and jumps on selection.

Desktop and web use the same GPUI view.

## Shape

The 36px / 44px glass track, each option and its stretchable selection use `Theme.tokens.radius.full`. They retain semicircular capsule ends while the two highlight edges move independently. This is an explicit exception to KUI's 8px rectangular surface rule (user instruction, 2026-11-05); it does not change the existing drag, keyboard or reduced-motion behavior.
