# Reply draft

Generate and edit a reply before inserting it into a host composer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add reply-draft`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::reply_draft::{ReplyDraft, ReplyHint, ReplyMessage};

let message = ReplyMessage::new("Alex", "Review", "Can we meet next week?");
let hints = vec![ReplyHint::new("yes", "Sounds good")];
let control = cx.new(|cx| ReplyDraft::new(message, hints, window, cx));
```

## Interaction

Choose a quick reply, tone or length. Stop or retry generation, edit the draft, then insert it. A result arriving during editing requires a choice.

## API

`append` · `(u64, &str, &mut Context<Self>) -> bool`

Handle Requested and Cancelled in the host; pass the request ID to append, finish or fail. Insert reports the edited text; the host controls insertion and sending.

Event: `ReplyDraftEvent`.

## Motion

Status fades in, while incoming text is displayed in a separate preview. Reduced motion shows the final state immediately.

Desktop and web use the same GPUI view.
