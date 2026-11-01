# CitationShelf

Browse references associated with an answer and inspect a source.

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
kui add source-cards
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::source_cards::{CitationShelf, Citation};

let control = cx.new(|_| CitationShelf::new("sources","References",vec![Citation {domain:"docs.rs".into(), title:"Rust API".into(), passage:"Public module documentation".into(),url:"https://docs.rs/".into()}]));
```

## Interaction

Inline citations reveal the corresponding source and overflow. Navigation is host-owned.

## API

`cite` · `(index, cx)`

Citation contains a domain, title, passage and URL. Use cite(index, cx) for inline references and SourceEvent to open or retry.

Event: `SourceEvent`.

## Motion

The grid keeps its outgoing cards while it folds; hovering a source for 320ms reveals its passage and later cards can be inspected immediately.

Desktop and web use the same GPUI view.
