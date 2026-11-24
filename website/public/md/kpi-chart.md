# KPI chart

Metric card with a rolling headline figure, delta badge, multi-series line/area chart, legend toggles, and a pointer-following tooltip.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add kpi-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::{prelude::*, rgb};
use kui::components::kpi_chart::{KpiChart, KpiChartEvent, KpiSeries};

let chart = cx.new(|cx| {
    KpiChart::new("revenue-kpi", cx)
        .with_title("Revenue")
        .with_labels(["Mar 3", "Mar 4", "Mar 5", "Mar 6", "Mar 7"])
        .with_series(vec![
            KpiSeries::new("organic", "Organic", rgb(0x4fb0fe).into(), vec![4200., 4700., 3900., 4600., 5400.]),
            KpiSeries::new("paid", "Paid ads", rgb(0xee3e9e).into(), vec![3600., 3300., 4400., 4800., 4000.]),
        ])
        .with_delta(Some(7_900.0))
        .with_footer("Top channels")
});
cx.subscribe(&chart, |_, ev: &KpiChartEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Move across the plot or press Left / Right arrow keys to scrub daily values in the floating tooltip, click any channel pill in the legend to toggle series on or off, and flip between Line and Area rendering from the header.

## API

Primary GPUI type: `KpiChart` (`kui::components::kpi_chart::KpiChart`).
Event type: `KpiChartEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Series curves draw in on mount, morph Y-axis ceilings and visibility via `pro_spring_f32`, roll headline totals via `pro_rolling_number`, and glide the vertical crosshair between points. Reduced motion disables sweeps and glides.

Desktop and web use the same GPUI source.
