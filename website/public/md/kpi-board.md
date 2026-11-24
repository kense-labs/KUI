# KPI board

Coordinate three headline trend cards, a period-over-period comparison chart, and a product focus card under one shared date-range selector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add kpi-board`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::kpi_board::{KpiBoardBlock, KpiBoardEvent};

let board = cx.new(|cx| {
    KpiBoardBlock::new("overview-kpi-board", cx)
        .with_default_range("30d")
        .with_compare_label("vs. last month")
});
cx.subscribe(&board, |_, event: &KpiBoardEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click or tab through the `7d`, `30d`, `90d`, and `12m` range selector to roll every headline figure and morph all sparklines and comparison lines together. Switch segments inside the overview card (`All plans`, `Pro`, `Team`) to update that card independently, hover any chart to scrub daily values, or press Left/Right/Home/End/Escape to step through days via keyboard.

## API

Primary GPUI type: `KpiBoardBlock` (`kui::components::kpi_board::KpiBoardBlock`).
Event type: `KpiBoardEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation (`894px` down to `390px`).

## Motion

Switching ranges rolls tabular figures (`pro_rolling_number`) while resampling and morphing sparklines and dashed/solid comparison curves smoothly. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
