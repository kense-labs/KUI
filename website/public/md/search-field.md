# Search field

Type a search query and clear it without shifting the toolbar.

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
kui add search-field
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::search_field::SearchField;

let control = cx.new(|cx| SearchField::new("query", "Search projects", window, cx).with_placeholder("Search by name", cx));
```

FieldEvent::Changed supplies the query, including the empty string after clearing.

## Interaction

Escape or Clear search resets the query. The clear action returns focus to the field. The clear icon scales inside a reserved slot.

- Event: `FieldEvent`.
- Preview: [English](/en/kui/components/search-field/) · [中文](/kui/components/search-field/).
