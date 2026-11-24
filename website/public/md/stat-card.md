# Stat card

KPI metric card with a rolling figure, directional delta badge (`lower_is_better` aware), embedded sparkline or bar slot below or beside the figure, and interactive link/tab selection.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add stat-card`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::stat_card::{
    StatCard, StatCardEvent, StatSparkKind, StatSparkSpec,
};

let spend = cx.new(|cx| {
    StatCard::single("spend-stat", "Spend", 2_741.0, cx)
        .with_format(1_000.0, 1, "$", "k")
        .with_change(16.5, "vs last week")
        .with_spark(
            StatSparkSpec::new(
                StatSparkKind::Area,
                ["Mar 3", "Mar 4", "Mar 5", "Mar 6"],
                vec![412., 455., 398., 470.],
            )
            .with_format(1.0, 0, "$", ""),
        )
});
cx.subscribe(&spend, |_, ev: &StatCardEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Switch 7D and 30D in the default 4-card Usage showcase (`Spend`, `Tokens`, `Avg. latency`, `Error rate`) to roll figures and morph sparklines together, click any card to spotlight it or follow its link, and hover across any sparkline or bar strip to read daily tags.

## API

Primary GPUI type: `StatCard` (`kui::components::stat_card::StatCard`).
Event type: `StatCardEvent`.

Supports `StatCardLayout::Stacked` and `StatCardLayout::Inline`, `with_lower_is_better(true)` for latency/error metrics, `with_href(...)`, `with_selected(Option<bool>)`, light and dark `Theme` modes, bilingual `UiLanguage`, `with_loading(true)` placeholders, and `with_empty(true)` empty states.

## Motion

Figures roll smoothly via `pro_rolling_number`, sparklines and bars resample and morph across periods via `PhaseClock`, and sibling cards dim smoothly via `pro_spring_f32` when one card is spotlighted. Reduced motion disables rolling offsets and curve interpolation.

Desktop and web use the same GPUI source.
