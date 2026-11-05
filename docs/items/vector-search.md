# Vector search

Explain a host-supplied vector search result with an illustrative projection.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add vector-search`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::vector_search::{VectorDocument, VectorMatch, VectorSearch, VectorSearchSnapshot};

let documents = vec![VectorDocument::new("doc", "Guide", "Access", 0.1, 0.2, gpui::rgb(0x9b84d6).into())];
let matches = vec![VectorMatch::new("doc", 0.9)];
let snapshot = VectorSearchSnapshot::new("Query", documents, matches);
let control = cx.new(|_| VectorSearch::new(snapshot));
```

## Interaction

The host supplies documents, projection coordinates, links, result order and scores. Coordinates are illustrative and do not calculate real similarity.

## API

`set_snapshot` · `(VectorSearchSnapshot, &mut Context<Self>)`

Use stage buttons or set_stage to inspect, set_paused for playback, and set_visible to stop off-screen animation.

Event: `VectorStageChanged, VectorPlaybackChanged`.

## Motion

The illustration progresses through embedding, indexing, query and results; reduced motion shows a still frame. The reference's 2.5D geometry and detailed transitions are not yet reproduced, and device/accessibility validation remains pending.

Desktop and web use the same GPUI view.
