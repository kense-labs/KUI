# ResizablePanels

Distribute horizontal space between adjacent panes.

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
kui add resizable-panels
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::resizable_panels::{ResizablePanels, Pane};

let control = cx.new(|cx| ResizablePanels::new("workspace", "Workspace", vec![
        Pane::new("files", "Files", 30., |_, _, _| gpui::div().child("Files")).collapsible(),
        Pane::new("editor", "Editor", 70., |_, _, _| gpui::div().child("Editor")),
    ], cx));
```

Give the group a definite height. Pane shares are relative weights. LayoutChanged contains settled percentages in pane order.

## Interaction

Drag a divider or use Left and Right in 16px steps. Shift increases the step to 64px. Enter collapses or restores a collapsible neighbour. Dragging follows the pointer. Limits resist and release springs the divider into place.

- Event: `LayoutChanged`.
- Preview: [English](/en/kui/components/resizable-panels/) · [中文](/kui/components/resizable-panels/).
