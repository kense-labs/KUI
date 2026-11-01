# MessageTranslation

Request a translation and compare it with the original message.

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
kui add ai-translate
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ai_translate::{MessageTranslation};

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
