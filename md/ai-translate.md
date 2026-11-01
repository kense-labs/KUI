# MessageTranslation

Request a translation and compare it with the original message.

## Install

```bash
kui add ai-translate
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::ai_translate::{MessageTranslation};

let control = cx.new(|_| MessageTranslation::new("translation","Maya","Bonjour !",vec!["English".into()]));
```

## Interaction

Keep the source available and apply only the current translation ticket.

## API

`update_translation` · `(ticket, Result<String, ()>, finished, cx)`

TranslationRequest carries a ticket and selected language. Send cumulative text by ticket, then mark the final update.

Event: `TranslationRequest`.

## Motion

New text ranges fade for 140ms independently. Completed translations cache by source and language, and late chunks are rejected by ticket.

Desktop and web use the same GPUI view.
