# Popover

Keep contextual information beside the control that opened it.

`kui add popover`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::popover::Popover;

let control = cx.new(|cx| Popover::new("checks", "Why this status?", "Ready for review", cx)
        .with_description("All required checks passed.").with_cancel("Got it"));
```

with_cancel adds a dismiss action. with_body mounts interactive host content.

## Interaction

Enter opens the panel. Escape returns to the trigger. Clicking outside dismisses. The surface fades and travels 5px from its anchor. It flips above near the bottom edge.

- Event: `SurfaceEvent`.
- Preview: [English](/en/kui/components/popover/) · [中文](/kui/components/popover/).
