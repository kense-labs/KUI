# CommandDesk

Choose and run an assistant command against a selection.

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
kui add ai-command
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ai_command::{CommandDesk, Command};

let control = cx.new(|cx| CommandDesk::new("desk",vec![Command{id:"inbox".into(),label:"Inbox".into(),section:"Pages".into(),hint:"G I".into()}],window,cx));
```

## Interaction

Filter host commands or submit a question. Changing mode invalidates the previous answer stream.

## API

`with_model(model)` displays a model label and its supported brand mark. When the editor has focus, Tab switches to Ask AI and submits a nonempty query. Escape returns from Ask AI to search and cancels any pending answer.

`receive` · `(ticket, Result<String, ()>, done, cx)`

Command provides ID, name, group and shortcut hint. CommandEvent separates navigation, question submission and cancellation.

Event: `CommandEvent`.

## Motion

Search and answer views share a fixed results area, crossfading with 2px travel. New streamed text ranges fade independently for 140ms.

Desktop and web use the same GPUI view.
