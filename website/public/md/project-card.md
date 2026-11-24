# Project card

Two-line project summary card that unfolds quietly on hover or click — pairing a multi-segment `PathBuilder` progress ring (one arc per subtask) with a teammate avatar filter, inline status picker, and cross-linked subtask list.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add project-card`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::project_card::{ProjectCard, ProjectCardEvent, ProjectStatus};

let card = cx.new(|cx| {
    ProjectCard::new("onboarding-project", cx)
        .with_title("Onboarding flow")
        .with_status(ProjectStatus::OnTrack)
        .with_due("Oct 24")
});
cx.subscribe(&card, |_, event: &ProjectCardEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover the card (or press `Enter` / `Space`) to unfold its subtasks and footer tray. Hover or click a ring segment or subtask row to cross-highlight its assignee; hover or click a teammate avatar to spotlight only that person's subtasks; click the status pill to unfold `On track`, `At risk`, `Off track`, and `Completed` options.

## API

Primary GPUI type: `ProjectCard` (`kui::components::project_card::ProjectCard`).
Event type: `ProjectCardEvent`.

Configure project metadata with `with_title`, `with_due`, `with_status`, `with_members`, `with_tasks(Vec<ProjectSubtask>)`, `with_action_label`, `with_trigger(ProjectCardTrigger)`, and `with_default_open(bool)`. Supports light and dark `Theme` modes, bilingual `UiLanguage`, narrow `<290px` responsive adaptation, `with_loading(true)`, and `with_empty(true)`.

## Motion

The two-line header stays anchored while `pro_fold` smoothly reveals the inline status strip, subtask list, and footer tray. Hovering a subtask or ring segment thickens its `PathBuilder` arc from `3px` to `4.25px` and glides in the assignee's avatar, while center completion counts roll smoothly. `Theme::reduced_motion` completes transitions immediately.

Desktop and web use the same GPUI source.
