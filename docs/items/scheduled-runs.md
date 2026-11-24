# Scheduled runs

Pack queued agent tasks across a week of quota-capped execution sessions — small tasks fill remaining session gaps without splitting, oversized tasks (`> 100%`) split cleanly across adjacent windows respecting `min_piece`, and cards stagger smoothly into place with interactive task addition, manual placement, and agent filtering.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add scheduled-runs`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::scheduled_runs::{
    ScheduledRuns, ScheduledRunsEvent, ScheduledTask, SessionSlot,
};

let board = cx.new(|cx| {
    ScheduledRuns::new("weekly-sessions", cx)
        .with_note("5-hour sessions")
        .with_now(SessionSlot::new(0, 1))
        .with_tasks(vec![
            ScheduledTask::new("metrics", "Weekly metrics report", "Gemini CLI", "gemini", 55)
                .with_booked_at(SessionSlot::new(0, 0)),
            ScheduledTask::new("oauth", "Migrate auth to OAuth 2.1", "Claude Code", "claude", 140),
            ScheduledTask::new("flaky", "Fix flaky checkout tests", "Claude Code", "claude", 40),
        ])
});
cx.subscribe(&board, |_, ev: &ScheduledRunsEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Toggle between `Auto` (automatic gap-filling and multi-session splitting) and `Manual` (keeping unbooked tasks in the left `To schedule` stack). Clicking any movable card selects it and highlights all upcoming sessions with room while striping full or past sessions; clicking a highlighted session books or moves the task into that window. Clicking agent logos in the header isolates that runner's tasks, `+ Add task` appends a new queued task, and `Rebalance` repacks all unbooked work.

## API

Primary GPUI type: `ScheduledRuns` (`kui::components::scheduled_runs::ScheduledRuns`).
Supporting types: `ScheduledTask`, `ScheduledPart`, `ScheduleDay`, `SessionSlot`, `ScheduleMode`, `WeekPlan`, `plan_week`.
Event type: `ScheduledRunsEvent` (`ModeChanged`, `Scheduled`, `TaskPlaced`, `TaskAdded`, `AgentFiltered`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`PhaseClock` and `pro_ease` stagger placed task cards smoothly from the stack into their target session slots while `pro_spring_bar` animates each session's bottom usage meter; `Theme::reduced_motion` settles cards and quota bars immediately.

Desktop and web use the same GPUI source.
