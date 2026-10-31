# Expanding search

Open a compact search button, browse recent items and choose from grouped results.

`kui add expanding-search`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::expanding_search::{ExpandingSearch, SearchItem};

let control = cx.new(|cx| ExpandingSearch::new("find", "Search", vec![SearchItem::new("notes", "Release notes")], window, cx).with_suggestions(vec!["notes".into()]));
```

SearchItem has a stable ID, title, detail, group and keywords. Choose a result through SearchEvent::Selected.

## Interaction

Open with Enter or Space. Type to filter. Arrow keys wrap, Enter selects and Escape closes. The field grows on a spring and one highlight follows the active row.

- Event: `SearchEvent`.
- Preview: [English](/en/kui/components/expanding-search/) · [中文](/kui/components/expanding-search/).
