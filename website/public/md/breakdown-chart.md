# Breakdown chart

Slice and stack categorical records across interactive `Measure`, `Slice`, and `Segment` pickers with spring-animated bars, a hover breakdown tooltip, and a bidirectionally linked table.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add breakdown-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::breakdown_chart::{BreakdownChart, BreakdownChartEvent};

let chart = cx.new(|cx| {
    BreakdownChart::new("sprint-breakdown", cx)
        .with_title("Sprint 24")
        .with_default_measure("count")
        .with_default_slice("status")
        .with_default_segment("priority")
});
cx.subscribe(&chart, |_, event: &BreakdownChartEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Open the `Measure`, `Slice`, or `Segment` pickers (`Escape` closes any open menu) to re-aggregate the dataset on the fly. Hover any bar column to inspect its per-segment breakdown tooltip, hover a segment row or legend chip below to highlight that segment across every bar (and vice versa), switch table orientation between `By segment` and `By slice`, or click legend chips to toggle individual segments with `Reset`.

## API

Primary GPUI type: `BreakdownChart` (`kui::components::breakdown_chart::BreakdownChart`).
Event type: `BreakdownChartEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_records(...)`, `with_dimensions(...)`, `with_measures(...)`, `with_default_measure(...)`, `with_default_slice(...)`, `with_default_segment(...)`, `with_table_mode(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Changing measures, dimensions, or visible legend segments morphs every stacked bar segment height smoothly via `pro_spring_f32`, rolls the grand total via `pro_rolling_number`, glides the table mode pill via `pro_pill_indicator`, and snaps immediately when `Theme::reduced_motion` is active.

Desktop and web use the same GPUI source.
