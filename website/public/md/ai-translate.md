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

Pick a language from the floating list. Use arrows and Enter to select, Escape to close, and Show original to return. Retry preserves the last readable message.


## API

`update_translation` · `(ticket, Result<String, ()>, finished, cx)`

TranslationRequest carries the ticket and target language. Call update_translation with cumulative text; mark the final update. Set credit to the actual model name and use byline and portrait for author details.

Event: `TranslationRequest`.


## Motion

New chunks replace the dimmed text sentence by sentence. Cached translations and the original crossfade. Stale tickets never overwrite the active request.

Desktop and web use the same GPUI view.
