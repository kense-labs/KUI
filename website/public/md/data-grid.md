# DataGrid

Inspect, sort and edit cells in a spreadsheet-like grid.

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
kui add data-grid
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::data_grid::DataGrid;

let control = cx.new(|cx| DataGrid::new("Renewals", vec!["Account".into(),"Seats".into()], vec![vec!["Northwind".into(),"40".into()]],window,cx));
```

## Interaction

Search rows, apply column filters, and show, hide or reorder columns before exporting. Arrows and Shift select a range. Double-click or Enter edits, Escape cancels, and dragging the corner fills.

## API

`csv` · `(&self) → String`

Subscribe to CellEdited for backing row/column updates. GridExport carries a file name and CSV in filtered row order and current visible column order. The host saves the CSV.

Event: `CellEdited, GridExport`.

## Motion

The visible region animates row identities after sorting and fades arriving rows. The range glides in scroll-content coordinates and written cells flash for 900ms.

Desktop and web use the same GPUI view.
