# Toast

Acknowledge a short-lived outcome with an optional follow-up action.

## Install

```bash
kui add toast
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::toast::Toast;

let control = cx.new(|_| Toast::new());
```

## Interaction

Call show on the entity. Hover pauses dismissal. Undo and dismiss are distinct events.

## API

`show` · `(message, Option<SharedString>, Duration)`

Call show on the entity. Hover pauses dismissal. Undo and dismiss are distinct events.

Event: `ToastEvent`.

## Motion

The notification rises 16px, draws its check and fades out while dropping 8px. Drag can dismiss it and hover pauses its timeout.

Desktop and web use the same GPUI view.
