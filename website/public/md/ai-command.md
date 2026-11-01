# CommandDesk

Choose and run an assistant command against a selection.

## Install

```bash
kui add ai-command
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::ai_command::{CommandDesk, Command};

let control = cx.new(|cx| CommandDesk::new("desk",vec![Command{id:"inbox".into(),label:"Inbox".into(),section:"Pages".into(),hint:"G I".into()}],window,cx));
```

## Interaction

Filter host commands or submit a question. Changing mode invalidates the previous answer stream.

## API

`receive` · `(ticket, Result<String, ()>, done, cx)`

Command provides ID, name, group and shortcut hint. CommandEvent separates navigation, question submission and cancellation.

Event: `CommandEvent`.

## Motion

Search and answer views share a fixed results area, crossfading with 2px travel. New streamed text ranges fade independently for 140ms.

Desktop and web use the same GPUI view.
