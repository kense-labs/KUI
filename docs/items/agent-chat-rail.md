# Agent chat rail

Pair a 52px primary icon rail and project-scoped chat list with web-grounded conversation and an interactive spreadsheet artifact viewer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-chat-rail`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_chat_rail::{AgentChatRailScreen, AgentChatRailEvent};

let screen = cx.new(|cx| AgentChatRailScreen::new("agent-chat-rail", cx));
cx.subscribe(&screen, |_, event: &AgentChatRailEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Cmd+K opens search, / focuses the composer, arrow keys navigate spreadsheet cells with live formula bar readout, and Delete closes artifact tabs.

## API

Primary GPUI type: `AgentChatRailScreen` (`kui::components::agent_chat_rail::AgentChatRailScreen`).
Event type: `AgentChatRailEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
