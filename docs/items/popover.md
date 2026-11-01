# Popover

Keep contextual information beside the control that opened it.

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
kui add popover
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::popover::Popover;

let control = cx.new(|cx| Popover::new("checks", "Why this status?", "Ready for review", cx)
        .with_description("All required checks passed.").with_cancel("Got it"));
```

with_cancel adds a dismiss action. with_body mounts interactive host content.

## Interaction

Enter opens the panel. Escape returns to the trigger. Clicking outside dismisses. The surface fades and travels 5px from its anchor. It flips above near the bottom edge.

- Event: `SurfaceEvent`.
- Preview: [English](/en/kui/components/popover/) · [中文](/kui/components/popover/).
