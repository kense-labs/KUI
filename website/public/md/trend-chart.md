# Trend chart

Versatile time-series chart—switchable between Line, Area, Bar, and Sparkline from the same series data, with period-over-period dashed overlays, hover scrubber tooltips, and synchronized day scrubbing across linked charts.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add trend-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::trend_chart::{
    TrendChart, TrendChartEvent, TrendSeries, TrendVariant,
};

let chart = cx.new(|cx| {
    TrendChart::standalone("spend-trend", cx)
        .with_variant(TrendVariant::Area)
        .with_label("Spend per day")
        .with_labels(["Mar 3", "Mar 4", "Mar 5", "Mar 6"])
        .with_format("$", "", 0)
        .with_series(vec![
            TrendSeries::new("this", "This week", vec![412., 455., 398., 470.]),
            TrendSeries::new("last", "Last week", vec![356., 380., 371., 390.])
                .with_previous(true),
        ])
});
cx.subscribe(&chart, |_, ev: &TrendChartEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Switch between Line, Area, and Bar or between 7D and 30D in the default AI spend showcase, and hover or press Left / Right / Home / End on any of the four linked charts to scrub the same day simultaneously across the primary chart and all three sparklines.

## API

Primary GPUI type: `TrendChart` (`kui::components::trend_chart::TrendChart`).
Event type: `TrendChartEvent`.

Supports `TrendVariant` (`Line`, `Area`, `Bar`, `Sparkline`), `TrendAxisMode` (`Both`, `XOnly`, `None`), `with_zero(false)` zoomed rate scales, `None` line gaps via `TrendSeries::with_gaps`, `with_controlled_index` multi-chart sync, light and dark `Theme` modes, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Lines and fills draw in on mount, series values resample smoothly across point counts via `PhaseClock`, Y-axis bounds spring via `pro_spring_f32`, and crosshair guides glide between days. Reduced motion snaps all transitions.

Desktop and web use the same GPUI source.
