# Tabs

Navigate peer views while preserving the application state of each panel.

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
kui add tabs
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tabs::{Tabs, TabPage};

let control = cx.new(|cx| Tabs::new("views", "Workspace views", vec![
        TabPage::new("overview", "Overview", |_, _| gpui::div().child("Workspace overview")),
        TabPage::new("activity", "Activity", |_, _| gpui::div().child("Recent activity")),
    ], cx));
```

Give each TabPage a stable value. Capture an existing Entity for a stateful panel.

## Interaction

Left and Right move between enabled tabs. Home and End jump to the edges. One selection surface glides across the rail. Incoming content slides 8px in the direction of travel.

- Event: `TabChanged`.
- Preview: [English](/en/kui/components/tabs/) · [中文](/kui/components/tabs/).
