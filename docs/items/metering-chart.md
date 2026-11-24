# Metering chart

Daily usage metering bars painted with soft blue-to-lilac gradients, a live growing bar for Today, a Real-time badge, per-day tool breakdown tooltips, and rolling period totals.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add metering-chart`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::metering_chart::{MeteringChart, MeteringChartEvent, MeteringDay, MeteringTotal};

let chart = cx.new(|cx| {
    MeteringChart::new("api-calls", cx)
        .with_title("API calls")
        .with_live(true)
        .with_period("Sep 1 – Sep 21")
        .with_data(vec![
            MeteringDay::new("Sep 1", 1870.),
            MeteringDay::new("Sep 2", 2290.),
            MeteringDay::new("Today", 2860.),
        ])
        .with_totals(vec![
            MeteringTotal::new("vol", "Total volume", 50.39).with_format(1.0, 2, "$", "k"),
            MeteringTotal::new("evt", "Total events", 36.0).with_format(1.0, 1, "", "k"),
        ])
});
cx.subscribe(&chart, |_, ev: &MeteringChartEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hover across daily bars or press Left, Right, Home, and End to inspect each day's usage count and tool share percentages, or trigger live API bursts to watch Today's bar and rolling totals grow in real time.

## API

Primary GPUI type: `MeteringChart` (`kui::components::metering_chart::MeteringChart`).
Event type: `MeteringChartEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Bars rise from left to right in a wave on mount, spring height changes via `pro_spring_f32`, sweep a subtle live sheen across Today's bar, and roll period totals via `pro_rolling_number`. Reduced motion disables sweeps, pulses, and wave staggering.

Desktop and web use the same GPUI source.
