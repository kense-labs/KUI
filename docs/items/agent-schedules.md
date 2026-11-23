# Agent schedules

Monitor recurring automations on an Around-now activity curve with capsule switches, Run now triggers, and upcoming/past run logs.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-schedules`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_schedules::{AgentSchedulesScreen, AgentSchedulesEvent};

let screen = cx.new(|cx| AgentSchedulesScreen::new("agent-schedules", cx));
cx.subscribe(&screen, |_, event: &AgentSchedulesEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K moves across schedules, Enter opens the right schedule inspector, Space toggles active/paused state, R triggers Run now, and Esc closes overlays.

## API

Primary GPUI type: `AgentSchedulesScreen` (`kui::components::agent_schedules::AgentSchedulesScreen`).
Event type: `AgentSchedulesEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
