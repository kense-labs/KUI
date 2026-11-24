# Data table

Spreadsheet-style table with rectangular cell-range selection, in-place editing, TSV copy and paste, undo/redo, and sorting.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add data-table`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::data_table::{DataColumn, DataColumnType, DataRow, DataTable, DataTableEvent};

let table = cx.new(|cx| {
    DataTable::new("design-stack", cx)
        .with_title("Design Stack")
        .with_columns(vec![
            DataColumn::new("tool", "Tool").with_type(DataColumnType::Title),
            DataColumn::new("category", "Category")
                .with_type(DataColumnType::Tag)
                .with_options(["Design", "Hosting", "Website"]),
            DataColumn::new("website", "Website").with_type(DataColumnType::Link),
            DataColumn::new("seats", "Seats").with_type(DataColumnType::Number),
        ])
        .with_rows(vec![
            DataRow::new("figma")
                .with_cell("tool", "Figma")
                .with_cell("category", "Design")
                .with_cell("website", "figma.com")
                .with_cell("seats", "24"),
        ])
});
cx.subscribe(&table, |_, ev: &DataTableEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Click and pointer-drag or press Shift+Arrow to select rectangular cell ranges, double-click or press Enter/typing to edit in place, copy and paste TSV across cells with Cmd+C / Cmd+V, undo and redo with Cmd+Z / Cmd+Shift+Z, and sort from column headers.

## API

Primary GPUI type: `DataTable` (`kui::components::data_table::DataTable`).
Event type: `DataTableEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and horizontal scroll containment on narrow viewports.

## Motion

Sort arrows orient cleanly, tag pickers settle over active cells, and reduced motion disables loading sweeps and transitions.

Desktop and web use the same GPUI source.
