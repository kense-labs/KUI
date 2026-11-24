# Parallel attempts

Race multiple models or solution strategies on the same task simultaneously — each attempt displays its provider mark, approach, and live `pro_spring_bar` meter, automatically crowns and selects the highest-scoring winner, and lets users inspect diffs or switch the chosen patch.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add parallel-attempts`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::parallel_attempts::{
    AttemptStatus, ParallelAttempt, ParallelAttempts, ParallelAttemptsEvent,
};

let attempts = cx.new(|cx| {
    ParallelAttempts::new("checkout-best-of-4", cx)
        .with_task("Fix the flaky checkout test")
        .with_attempts(vec![
            ParallelAttempt::new("a", "Claude Opus 4.6", "Idempotency key + backoff")
                .with_icon("claude")
                .with_status(AttemptStatus::Done)
                .with_score(96)
                .with_note("20/20 stress runs green")
                .with_cost(0.26),
            ParallelAttempt::new("b", "GPT-5", "Deterministic mock clock")
                .with_icon("openai")
                .with_status(AttemptStatus::Running)
                .with_progress(0.64)
                .with_step("Running tests 13/20"),
        ])
});
cx.subscribe(&attempts, |_, ev: &ParallelAttemptsEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

The highest-scoring `Done` attempt automatically receives the `Best` badge and soft green active wash while non-selected rows step back softly. Hovering any finished row reveals `Use this one`; clicking any row inspects its unified diff below and (if `Done`) switches the active winner, while `Re-run` replays the live parallel race from `0%`.

## API

Primary GPUI type: `ParallelAttempts` (`kui::components::parallel_attempts::ParallelAttempts`).
Supporting types: `ParallelAttempt`, `AttemptStatus`, `find_best_attempt_id`.
Event type: `ParallelAttemptsEvent` (`Pick(ParallelAttempt)`, `Inspect(ParallelAttempt)`, `RaceRestarted`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton rows, and `with_empty(true)` empty states.

## Motion

`pro_spring_bar` grows smoothly with a sweeping specular shine on `Running` rows, rolling summary numbers interpolate completion and spend, and `pro_view_transition` cross-glides the code diff preview when switching attempts; `Theme::reduced_motion` settles races and transitions immediately.

Desktop and web use the same GPUI source.
