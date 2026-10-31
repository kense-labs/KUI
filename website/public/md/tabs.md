# Tabs

Navigate peer views while preserving the application state of each panel.

`kui add tabs`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::tabs::{Tabs, TabPage};

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
