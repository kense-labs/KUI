# Run controls

Every control a running agent needs in one composer—stop mid-sentence, resume from the exact word where it stopped, retry or dismiss after a drop, steer mid-answer or queue what comes next, switch `Chat` vs `Act` mode, pick permission posture, and monitor context-window fill.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add run-controls`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::run_controls::{RunControls, RunControlsEvent, RunMode, RunPermission};

let controls = cx.new(|cx| {
    RunControls::new("assistant-run-controls", cx)
        .with_agent("travel", "Trip planner")
        .with_mode(RunMode::Act)
        .with_permission(RunPermission::AskFirst)
        .with_context_used(0.44)
});
cx.subscribe(&controls, |_, event: &RunControlsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press `Esc` (or the Stop button) while the agent writes to pause at the exact word index, then press `Resume` to continue seamlessly from that word. While streaming, typing in the `Editor` and pressing `Enter` injects an inline steer pill right where the stream stands; pressing `⌥↵` (or clicking `Queue`) lines the message up in the tray above the composer. Switch between `Chat` and `Act` with clicks or `←` / `→`, and choose `Ask first`, `Auto-edit`, or `Full access` from the permissions menu.

## API

Primary GPUI type: `RunControls` (`kui::components::run_controls::RunControls`).
Event type: `RunControlsEvent`.

Supports light/dark `Theme` modes, bilingual `UiLanguage`, loading skeletons, empty states, and compact adaptation below `420px`.

## Motion

The 3x3 pixel agent mark twinkles while writing, the `Chat` / `Act` pill glides with `pro_pill_indicator`, the permission menu opens with `pro_presence`, and the context ring sweeps smoothly via `pro_progress_ring`; `Theme::reduced_motion` disables continuous pulses.

Desktop and web use the same GPUI source.
