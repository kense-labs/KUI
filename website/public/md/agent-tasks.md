# Agent tasks

Supervise active and completed agent tasks grouped by project or status with progress rings, inline asks, and multi-tab artifacts.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-tasks`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_tasks::{AgentTasksScreen, AgentTasksEvent};

let screen = cx.new(|cx| AgentTasksScreen::new("agent-tasks", cx));
cx.subscribe(&screen, |_, event: &AgentTasksEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K navigates tasks, Enter opens the task inspector, N or C focuses the new-task composer, and artifact chips open directly in the right work area.

## API

Primary GPUI type: `AgentTasksScreen` (`kui::components::agent_tasks::AgentTasksScreen`).
Event type: `AgentTasksEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
