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

Press Enter or the submit button to run a question. Press Escape while editing the question, or use the Stop button, to cancel a pending request. Inspect or copy the generated SQL, then switch between bars, lines and a table. Hover a chart period or use the arrow keys with the chart focused to inspect its values. A failed request can be retried with the current question. The preview uses fixed sample data.

## API

While the host writes SQL or runs a query, the chart reserves space for the incoming result.

- `with_dialect(self, dialect: impl Into<SharedString>) -> Self` sets the optional SQL dialect shown in the generated query header. By default, no dialect is set.
- `set_stacked(&mut self, stacked: bool, cx: &mut Context<Self>)` switches between stacked and grouped columns. Columns are stacked by default.
- `finish_query(&mut self, id: u64, result: AskDataResult, cx: &mut Context<Self>) -> bool` accepts the result for the current request and returns whether it was applied.

The host handles GenerateSql, RunQuery and Cancelled. Pass matching IDs to append_sql, finish_sql, progress, finish_query or fail. The component does not connect to a database.

Event: `AskDataEvent`.

## Motion

Bars grow and lines are progressively drawn; reduced motion displays the final result immediately.

Desktop and web use the same GPUI view.
