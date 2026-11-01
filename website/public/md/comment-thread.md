# CommentThread

Read, reply to and resolve a discussion in context.

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
kui add comment-thread
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::comment_thread::{CommentThread, Comment};

let control = cx.new(|cx| CommentThread::new("review","Release notes",vec![Comment{key:1,parent:None,author:"Maya".into(),text:"Check the date.".into(),age:"2h".into(),likes:0,mine:false}],window,cx));
```

## Interaction

Use stable message keys for replies, edits and removals. Resolving closes the composer.

## API

`new` · `(id, subject, Vec<Comment>, window, cx)`

Comment uses a stable key and optional parent key. Apply CommentEvent mutations to the host-owned conversation.

Event: `CommentEvent`.

## Motion

Each reply retains its content during exit and clips to its measured height. Rapid changes reverse from the displayed height and velocity.

Desktop and web use the same GPUI view.
