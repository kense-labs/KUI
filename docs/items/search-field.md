# Search field

Type a search query and clear it without shifting the toolbar.

`kui add search-field`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::search_field::SearchField;

let control = cx.new(|cx| SearchField::new("query", "Search projects", window, cx).with_placeholder("Search by name", cx));
```

FieldEvent::Changed supplies the query, including the empty string after clearing.

## Interaction

Escape or Clear search resets the query. The clear action returns focus to the field. The clear icon scales inside a reserved slot.

- Event: `FieldEvent`.
- Preview: [English](/en/kui/components/search-field/) · [中文](/kui/components/search-field/).
