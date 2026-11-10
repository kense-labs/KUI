# Enrich column

Run a host-provided enrichment task for each row in an editable table.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add enrich-column`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::enrich_column::{EnrichColumn, EnrichRow};

let rows = vec![EnrichRow::new("one", "Northstar").with_domain("northstar.example")];
let control = cx.new(|cx| EnrichColumn::new("Accounts", rows, "Find a description", window, cx));
```

## Interaction

Edit the instruction, run all rows or one row, stop a run, and retry individual failures. Existing edits are protected; a result for a focused field waits for confirmation.

## API

Rows for known companies display the mark associated with `EnrichRow::name`. Hosts can still supply names and domains for arbitrary accounts.

`finish` · `(u64, EnrichResult, &Window, &mut Context<Self>) -> bool`

Handle EnrichEvent::Requested and Cancelled in the host. Deliver matching IDs to finish or fail, persist Changed events, and handle OpenSource in the host. Concurrency defaults to two.

Event: `EnrichEvent`.

## Motion

Cell states fade in and the instruction editor expands on a spring. Reduced motion suppresses movement.

Desktop and web use the same GPUI view.
