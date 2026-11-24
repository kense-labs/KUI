# Progress pill

Render a slim floating agent progress capsule with a twinkling pixel mark, report title, percentage readout, step subtitle, and ambient under-glow.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add progress-pill`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::progress_pill::{ProgressPill, ProgressPillEvent};

let pill = cx.new(|cx| {
    ProgressPill::new("q3-report-pill", cx)
        .with_title("Q3 revenue report")
        .with_name("Analyst")
        .with_steps([
            "Read the brief",
            "Pull the numbers",
            "Write the summary",
        ])
});
cx.subscribe(&pill, |_, ev: &ProgressPillEvent, _| {
    let _ = ev;
})
.detach();
```

## Interaction

Self-playing illustration with optional controlled `with_progress(Some(ratio))` and `with_playing(false)` modes; emits `ProgressPillEvent::StepChanged` and `ProgressPillEvent::Completed` as steps advance.

## API

Primary GPUI type: `ProgressPill` (`kui::components::progress_pill::ProgressPill`).
Event type: `ProgressPillEvent` (`kui::components::progress_pill::ProgressPillEvent`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_name(...)`, `with_steps(...)`, `with_agent(...)`, `with_step(...)`, `with_hold(...)`, `with_progress(...)`, `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Cycles smoothly through each step (`2.6s` hold by default), crossfading and sliding the current step subtitle into view while advancing the slim rounded progress bar with a traveling sheen, twinkling the 3×3 mirrored agent pixel avatar, and breathing a tri-color under-glow beneath the right-fading capsule; `Theme::reduced_motion` holds a calm static step pose.

Desktop and web use the same GPUI source.
