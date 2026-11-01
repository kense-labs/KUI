# PromptShelf

Start a new conversation from a short list of suggested questions.

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
kui add starter-prompts
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::starter_prompts::{PromptShelf, Prompt};

let control = cx.new(|cx| PromptShelf::new("prompts", "What shall we do?", vec![Prompt { group: "Write".into(), app: "Mail".into(), title: "Draft a reply".into(), detail: "Confirm the release".into(), text: "Draft a release reply.".into() }], window, cx));
```

## Interaction

Filter or shuffle prompts. Picking a card fills and focuses the editor.

## API

`new` · `(id, greeting, Vec<Prompt>, window, cx)`

Populate Prompt with its group, app, title, detail and draft text. PromptSent carries a trimmed draft.

Event: `PromptSent`.

## Motion

App, title and helper copy crossfade inside each card. Consecutive card slots delay by 35ms, and a second shuffle retains the interrupted label mixture.

Desktop and web use the same GPUI view.
