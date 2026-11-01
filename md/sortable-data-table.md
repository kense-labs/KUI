# SortableTable

Sort records by a column while keeping each row’s identity.

## Install

```bash
kui add sortable-data-table
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::sortable_data_table::{SortableTable, TableColumn};

let control = cx.new(|_| SortableTable::new("Scores", vec![TableColumn::new("Score",true)], vec![vec!["92".into()],vec!["8".into()]]));
```

## Interaction

Activate a column twice to reverse its order. Numeric columns compare parsed numbers.

## API

`sort` · `(&mut self, column: usize, descending: bool, &mut Context<Self>)`

Provide column definitions and ordered row values. Sorting announces the column and direction.

Event: `TableSorted`.

## Motion

Stable row identities carry the row-position spring across a second sort; the active heading arrow rotates.

Desktop and web use the same GPUI view.
