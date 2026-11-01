# Expanding search

Open a compact search button, browse recent items and choose from grouped results.

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
kui add expanding-search
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::expanding_search::{ExpandingSearch, SearchItem};

let control = cx.new(|cx| ExpandingSearch::new("find", "Search", vec![SearchItem::new("notes", "Release notes")], window, cx).with_suggestions(vec!["notes".into()]));
```

SearchItem has a stable ID, title, detail, group and keywords. Choose a result through SearchEvent::Selected.

## Interaction

Open with Enter or Space. Type to filter. Arrow keys wrap, Enter selects and Escape closes. The field grows on a spring and one highlight follows the active row.

- Event: `SearchEvent`.
- Preview: [English](/en/kui/components/expanding-search/) · [中文](/kui/components/expanding-search/).
