# Agent inbox board

Track background agent tasks across Needs you, Running, and Done columns with live status rings and detail inspector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-inbox-board`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_inbox_board::{AgentInboxBoardScreen, AgentInboxBoardEvent};

let screen = cx.new(|cx| AgentInboxBoardScreen::new("agent-inbox-board", cx));
cx.subscribe(&screen, |_, event: &AgentInboxBoardEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K navigates tasks, Enter inspects, A approves, X rejects, R retries failed runs, N opens New task, Left/Right toggles Board and List, and / filters.

## API

Primary GPUI type: `AgentInboxBoardScreen` (`kui::components::agent_inbox_board::AgentInboxBoardScreen`).
Event type: `AgentInboxBoardEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
