# Delegate board

Dispatch unassigned tasks onto specialist AI agents by drag-and-drop, click, or keyboard while each agent previews its updated `"Busy until …"` clock, timeline bar, and USD spend—or flip `Auto` (`pro_switch`) to route work automatically to whichever agent frees up earliest.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add delegate-board`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::delegate_board::{
    DelegateAgent, DelegateBoard, DelegateBoardEvent, DelegateTask,
};

let board = cx.new(|cx| {
    DelegateBoard::new("team-dispatch", cx)
        .with_agents(vec![
            DelegateAgent::new("coder", "Coder").with_model("GPT-6 Sol").with_provider("openai").with_rate(0.06),
            DelegateAgent::new("ops", "Ops").with_model("Gemini 3.8 Flash").with_provider("gemini").with_rate(0.01),
        ])
        .with_tasks(vec![
            DelegateTask::new("t1", "Fix flaky checkout test", 20),
        ])
});
cx.subscribe(&board, |_, ev: &DelegateBoardEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

- Drag any card from `Unassigned` onto an agent row (or click a task to pick it up, move across agents with hover or `ArrowUp` / `ArrowDown`, and press `Enter` or keys `1`–`4`) while the targeted agent previews `+$cost`, `+Xm until HH:MM`, and its extended timeline bar.
- Drag or click any non-running queued chip inside an agent's queue to return it to `Unassigned` for rebalancing.
- Flip the `Auto` capsule switch (`pro_switch`) to route unassigned tasks one by one to whichever agent frees up earliest.

## API

Primary GPUI type: `DelegateBoard` (`kui::components::delegate_board::DelegateBoard`).
Supporting types: `DelegateAgent`, `DelegateTask`, `DelegateMode`, `DelegateSuggestion`, `agent_busy_minutes`, `earliest_free_agent_id`, `format_busy_clock`.
Event type: `DelegateBoardEvent` (`Assigned { task_id, agent_id }`, `ModeChanged`, `TaskAdded`, `Reset`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`pro_switch` animates the capsule `Auto` thumb, `pro_spring_f32` grows each agent's busy and previewed load timeline bar, `pro_rolling_number` rolls total queue spend, and running task chips spin a live `PathBuilder` arc; `Theme::reduced_motion` completes `Auto` routing immediately without continuous loops.

Desktop and web use the same GPUI source.
