# Tree table

Browse, sort and select hierarchical rows with columns.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add tree-table`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tree_table::{TreeTable, TreeTableRow};

let rows = vec![TreeTableRow::new("team", "Team").with_children(vec![TreeTableRow::new("project", "Project")])];
let columns = vec!["Name".into(), "Owner".into()];
let control = cx.new(|cx| TreeTable::new("Projects", columns, rows, cx).with_expanded(&["team"]));
```

## Interaction

Expand a parent or use arrow keys to navigate. The header selects or expands all known rows. A failed lazy load can be retried by activating its row.

## API

`resolve_children` · `(&str, Result<Vec<TreeTableRow>, ()>, &mut Context<Self>)`

Provide unique stable IDs and column names. LoadRequested asks the host for lazy children; call resolve_children with success or failure. SelectionChanged returns leaf IDs and ExpandedChanged returns expanded IDs. Visible rows are virtualized; very large trees should still load lazily.

Event: `TreeTableEvent`.

## Motion

Nested rows expand and collapse on a spring; reduced motion applies changes immediately.

Desktop and web use the same GPUI view.
