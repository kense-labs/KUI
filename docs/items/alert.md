# Alert

Keep a persistent status message and its recovery action together.

## Install

```bash
kui add alert
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::alert::{Alert, Severity};

let control = cx.new(|_| Alert::new("Sync paused", "Reconnect to upload changes.", Severity::Warning).with_action("Reconnect"));
```

## Interaction

Offer one recovery action for a persistent status.

## API

`replace` · `Alert`

Offer one recovery action for a persistent status.

Event: `AlertAction`.

## Motion

Changing severity reveals the incoming symbol while the previous one recedes. Heading and detail rise and fade independently.

Desktop and web use the same GPUI view.
