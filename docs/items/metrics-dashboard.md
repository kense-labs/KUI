# Metrics dashboard

Overview sales and commerce performance with metric-selecting KPI cards, an area-or-bars trend chart, and spring-bar product and channel rankings.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add metrics-dashboard`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::metrics_dashboard::{
    DashboardMetric, DashboardRange, MetricsDashboardBlock, MetricsDashboardEvent,
};

let dashboard = cx.new(|cx| {
    MetricsDashboardBlock::new("sales-dashboard", cx)
        .with_title("Sales")
        .with_site("Northwind Store")
        .with_default_range(DashboardRange::Month)
        .with_default_metric(DashboardMetric::Revenue)
});
cx.subscribe(&dashboard, |_, event: &MetricsDashboardEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch between `Week`, `Month`, and `Quarter` to roll all KPI figures and ranked rows, click any KPI card (`Revenue`, `Orders`, `Avg. order`, `Refund rate`) to focus its series in the central `TrendChart`, toggle between `Area` and `Bars` or enable `Compare to previous period`, and hover or press Left/Right/Home/End/Escape to inspect individual days.

## API

Primary GPUI type: `MetricsDashboardBlock` (`kui::components::metrics_dashboard::MetricsDashboardBlock`).
Event type: `MetricsDashboardEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation (`894px` down to `390px`).

## Motion

Selected KPI cards highlight crisply while inactive cards dim, rolling counters interpolate with tabular numerals, area lines and bars morph smoothly between series, and ranked breakdown bars spring to relative shares (`pro_spring_f32`). Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
