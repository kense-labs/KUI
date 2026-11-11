# Ask your data

Ask a question and inspect a host-provided query and result.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add ask-your-data`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ask_your_data::AskYourData;

let control = cx.new(|cx| AskYourData::new("Analytics", window, cx));
```

## Interaction

Press Enter or the submit button to run a question. Escape in the question field or the Stop button cancels a pending request. Open the generated SQL to inspect it, or copy it directly from the SQL header. Use Left, Right, Home, or End on the result switch to select a view, or on the chart to inspect a period. Escape clears the chart readout. Retry uses the current question. The preview uses fixed sample data.

## API

While the host writes SQL or runs a query, the chart reserves space for the incoming result.

- `with_dialect(self, dialect: impl Into<SharedString>) -> Self` sets the optional SQL dialect shown in the generated query header. By default, no dialect is set.
- `with_suggestions(self, suggestions: Vec<SharedString>) -> Self` supplies starter prompts. The default list is empty.
- `with_keep_sql_open(self, keep_open: bool) -> Self` keeps the panel open at query start. The default is `false`. An explicit user choice takes priority.
- `reserve_sql_lines(&mut self, id: u64, lines: usize, cx: &mut Context<Self>) -> bool` reserves room before revealing a known response. For an unknown-length stream, append chunks as they arrive.
- `set_stacked(&mut self, stacked: bool, cx: &mut Context<Self>)` switches between stacked and grouped columns. The default is stacked, with totals in the table and chart readout.
- `finish_query(&mut self, id: u64, result: AskDataResult, cx: &mut Context<Self>) -> bool` accepts the result for the current request and returns whether it was applied.
- `finish_query_presented(&mut self, id: u64, result: AskDataResult, presentation: AskDataPresentation, cx: &mut Context<Self>) -> bool` also accepts an optional subtitle, category heading, opening view and number formatter. `AskDataLegend::Auto` displays sums when stacked. Choose `Sum`, `Last` or `None` to override it.

The host handles `GenerateSql`, `RunQuery` and `Cancelled`. Pass matching IDs to `append_sql`, `finish_sql`, `progress`, `finish_query` or `fail`. The host owns query validation, a read-only database role and the execution timeout.

Event: `AskDataEvent`.

## Motion

The SQL panel settles over approximately 300ms. Streamed SQL chunks fade in over 140ms. The fixed sample sends four words every 34ms. Bars rise over 320ms with up to 180ms stagger, and lines draw over 460ms. Changing views fades out over 100ms and back in over 160ms within the same 216px result area. Reduced motion settles the SQL panel and result immediately.

Desktop and web use the same GPUI view.
