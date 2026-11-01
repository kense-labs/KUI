# CitationShelf

Browse references associated with an answer and inspect a source.

## Install

```bash
kui add source-cards
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::source_cards::{CitationShelf, Citation};

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
