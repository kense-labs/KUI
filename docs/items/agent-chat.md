# Agent chat

Collaborate with a multi-agent creative team in chat alongside a structured campaign brief, audience pills, channel switches, and ad generator.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-chat`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_chat::{AgentChatScreen, AgentChatEvent};

let screen = cx.new(|cx| AgentChatScreen::new("agent-chat", cx));
cx.subscribe(&screen, |_, event: &AgentChatEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press / to focus the orchestrator composer, Cmd+Enter or Enter to submit, toggle reasoning traces inline, and switch Brief, Assets, and Activity tabs.

## API

Primary GPUI type: `AgentChatScreen` (`kui::components::agent_chat::AgentChatScreen`).
Event type: `AgentChatEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
