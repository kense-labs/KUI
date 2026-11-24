# Running now

Surface active background agent work in a header or sidebar with one quiet pill—`"3 running · 1 needs you"`—and open it into a rich popover with filling progress rings, current steps & sub-steps, `Pause` / `Resume`, and `Open` on tasks waiting for human input.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add running-now`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::running_now::{RunningNow, RunningNowAlign, RunningNowEvent};

let indicator = cx.new(|cx| {
    RunningNow::new("header-running-now", cx)
        .with_align(RunningNowAlign::End)
        .with_open(true)
});
cx.subscribe(&indicator, |_, event: &RunningNowEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click the trigger pill (or press `Enter` / `Space` / `↓`) to open or close the `348px` background tasks popover. Click any row to inspect its ordered sub-steps, press `Pause` or `Resume` on active/paused tasks without swapping row positions, click `Open` on `Needs you` blockers, or press `Esc` to close the popover.

## API

Primary GPUI type: `RunningNow` (`kui::components::running_now::RunningNow`).
Event type: `RunningNowEvent`.

Pass `.with_header_chrome(false)` when embedding the bare trigger pill inside your own header or sidebar. Supports light/dark `Theme` modes, bilingual `UiLanguage`, loading skeletons, empty states, and narrow-width pill compaction.

## Motion

Trigger counts roll smoothly via `pro_rolling_number`, the popover opens and closes via `pro_presence`, progress rings sweep via `pro_progress_ring`, step labels crossfade via `pro_view_transition`, and sub-steps expand via `pro_fold`; `Theme::reduced_motion` disables continuous pulses.

Desktop and web use the same GPUI source.
