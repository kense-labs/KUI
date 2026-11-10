# TL;DR toggle

Switch between an article, a host-generated summary and source-linked key points.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add tldr-toggle`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tldr_toggle::{TldrArticle, TldrParagraph, TldrToggle};

let article = TldrArticle::new("Project notes", vec![TldrParagraph::new("intro", "The source paragraph.")]);
let control = cx.new(|cx| TldrToggle::new(article, cx));
```

## Interaction

Select a view and length to request a summary. Stop or retry a run; activate a key point to focus its source paragraph. The preview uses sample data and sends nothing.

## API

Set the publisher with `TldrArticle::with_source` and the host-selected summarization model with `TldrToggle::with_model`.

`append_text` · `(u64, &str, &mut Context<Self>) -> bool`

Handle TldrEvent::Requested and Cancelled in the host. Pass matching request IDs to append_text, append_point, finish or fail; late events from cancelled requests are ignored. Successful results are cached by mode and length.

Event: `TldrEvent`.

## Motion

Content height follows its measured content on a spring; the incoming view fades in and the source highlight fades away. Reduced motion applies the final state immediately.

Desktop and web use the same GPUI view.
