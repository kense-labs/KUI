# ChatThread

Read a conversation and append a new message in sequence.

## Install

```bash
kui add chat-thread
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::chat_thread::{ChatThread, Message};

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
