# JsonViewer

Search, expand and inspect a parsed JSON value.

## Install

```bash
kui add json-viewer
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::json_viewer::{JsonViewer, JsonValue};

let control = cx.new(|cx| JsonViewer::new("payload", r#"{"status":"ok","items":[1,2]}"#.parse::<JsonValue>().unwrap(),window,cx));
```

## Interaction

Search keys and values without hiding their siblings. Enter and Shift+Enter move between matches, Escape restores prior branches, and Down moves from search into the tree. Arrow keys, Home and End navigate the rows. Cmd/Ctrl+C copies the value; add Shift to copy its display path. Expand all and Collapse all control the branches; long collections load 50 children per page.

## API

`new` · `(title, JsonValue, &mut Window, &mut Context<Self>) → Self`

Supply a parsed JsonValue. With serde_json’s preserve_order feature, object properties retain their source order. with_max_height controls the scroll viewport (420px by default). with_font_family uses a registered monospace font. selected_path returns a readable path; selected_pointer and JsonCopied.pointer use RFC 6901. String values copy without quotes; containers are copied as indented JSON.

Event: `JsonCopied`.

## Motion

Rows unfold and fold through their measured position. The active row, chevron and scroll follow a reversible spring. Above 400 rows, height changes are immediate; only text near the viewport is painted. Reduced motion uses the settled geometry.

Desktop and web use the same GPUI view.
