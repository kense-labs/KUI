# Fan-out grid

Render a multi-worker batch job as a dense live status matrix (`Queued`, `Running`, `Retrying`, `Done`, `Failed`) with rolling completion, cost, and ETA telemetry, status-filter pills, failed-batch retry, and a floating cell inspector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add fan-out-grid`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::fan_out_grid::{
    FanOutGrid, FanOutGridEvent, FanOutStatus, FanOutTask,
};

let grid = cx.new(|cx| {
    FanOutGrid::new("lead-enrichment", cx)
        .with_job("Enrich 240 leads")
        .with_workers(16)
        .with_tasks(vec![
            FanOutTask::new("t1", "linear.app").with_status(FanOutStatus::Done).with_timing(0, 1_180).with_cost(0.014).with_result("Series B · ICP score 94%"),
            FanOutTask::new("t2", "stripe.com").with_status(FanOutStatus::Running).with_start_ms(1_200).with_worker("worker-02"),
            FanOutTask::new("t3", "vercel.com").with_status(FanOutStatus::Failed).with_attempts(3).with_error("429 rate_limit_exceeded"),
        ])
});
cx.subscribe(&grid, |_, ev: &FanOutGridEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hovering any cell or navigating with `ArrowLeft`, `ArrowRight`, `ArrowUp`, `ArrowDown`, `Home`, or `End` pops the cell to `1.3x` scale and opens the floating task detail card without shifting layout. Clicking a cell or pressing `Enter` / `Space` pins the detail card (`Escape` releases it). Clicking status counts in the legend tray isolates a single status, and clicking `Retry N failed` requeues failed cells with incremented attempt counters.

## API

Primary GPUI type: `FanOutGrid` (`kui::components::fan_out_grid::FanOutGrid`).
Supporting types: `FanOutTask`, `FanOutStatus`, `FanOutCounts`, `compute_fan_out_counts`.
Event type: `FanOutGridEvent` (`Select(Option<FanOutTask>)`, `Retry { ids }`, `FilterChanged(Option<FanOutStatus>)`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Cells enter in a quick diagonal wave, `Running` cells sweep an out-of-phase specular sheen, `Retrying` cells breathe in amber, and rolling numeric counters interpolate smoothly; `Theme::reduced_motion` renders static cells and instant updates.

Desktop and web use the same GPUI source.
