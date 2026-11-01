# DataGrid

Inspect, sort and edit cells in a spreadsheet-like grid.

## Install

```bash
kui add data-grid
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::data_grid::DataGrid;

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
