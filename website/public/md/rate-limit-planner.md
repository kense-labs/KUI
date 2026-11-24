# Rate limit planner

Schedule recurring agent runs across a 24-hour API token-per-minute (`TPM`) ceiling — aggregate load is plotted via `PathBuilder` with diagonal red hatching wherever overlapping peaks exceed the dashed limit, bars drag directly across hours, and `Spread out` eliminates overflow peaks in one click.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add rate-limit-planner`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::rate_limit_planner::{
    PlannedRun, RateLimitPlanner, RateLimitPlannerEvent,
};

let planner = cx.new(|cx| {
    RateLimitPlanner::new("org-tpm-planner", cx)
        .with_limit(120_000.0)
        .with_runs(vec![
            PlannedRun::new("triage", "Inbox triage", 8 * 60, 46_000.0).with_pinned_running(true),
            PlannedRun::new("research", "Lead research", 11 * 60, 82_000.0).with_color_idx(1),
            PlannedRun::new("report", "Report writer", 11 * 60 + 30, 74_000.0).with_color_idx(2),
        ])
});
cx.subscribe(&planner, |_, ev: &RateLimitPlannerEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Drag any movable horizontal schedule bar left or right (or focus a bar and press `ArrowLeft` / `ArrowRight` for `±15m`, `Shift + ArrowLeft/Right` for `±60m`) to reschedule its start time and watch the 24-hour aggregate curve and red diagonal overflow hatch update live. Clicking `Spread out` shifts overlapping jobs apart while keeping in-flight `pinned_running` jobs anchored; `+ Add run` places a new job in the lowest-load time valley.

## API

Primary GPUI type: `RateLimitPlanner` (`kui::components::rate_limit_planner::RateLimitPlanner`).
Supporting types: `PlannedRun`, `LimitHealth`, `LoadSummary`, `compute_load_summary`, `format_clock_hhmm`.
Event type: `RateLimitPlannerEvent` (`RunsChanged`, `SelectedChanged`, `SpreadOut`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`Spread out` and keyboard shifts glide schedule bars and morph the 24-hour curve via `pro_spring_f32`, pointer drags track directly without lag, and pinned running agent marks twinkle; `Theme::reduced_motion` settles changes immediately.

Desktop and web use the same GPUI source.
