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

Submit a question, inspect or copy the generated SQL, then switch between bars, lines and a table. Stop and retry are available. The preview uses fixed sample data.

## API

`finish_query` · `(u64, AskDataResult, &mut Context<Self>) -> bool`

The host handles GenerateSql, RunQuery and Cancelled. Pass matching IDs to append_sql, finish_sql, progress, finish_query or fail. The component does not connect to a database.

Event: `AskDataEvent`.

## Motion

Bars grow and lines are progressively drawn; reduced motion displays the final result immediately.

Desktop and web use the same GPUI view.
