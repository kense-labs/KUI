# ChatThread

Read a conversation and append a new message in sequence.

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
kui add chat-thread
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::chat_thread::{ChatThread, Message};

let control = cx.new(|cx| ChatThread::new("thread", "Release review", vec![Message { author: "Maya".into(), time: "09:18".into(), text: "Ready for review.".into(), own: false, attachment: None, reactions: 0 }], window, cx));
```

## Interaction

Group adjacent messages by author. Sending requires a nonempty draft and online presence.

## API

`new` · `(id, topic, Vec<Message>, window, cx)`

Message supplies author, time, body, ownership, an optional attachment and count. Handle Send, Attach, Download and React in the host.

Event: `ChatEvent`.

## Motion

Counts crossfade their old and new values over 100/160ms. New messages enter from 4px and follow the bottom only if the reader is already there.

Desktop and web use the same GPUI view.
