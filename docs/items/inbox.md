# Agent inbox

Triage decisions, layout picks, approvals, and clarifying questions waiting on human sign-off alongside task artifacts.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add inbox`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::inbox::{AgentInboxScreen, AgentInboxEvent};

let screen = cx.new(|cx| AgentInboxScreen::new("inbox", cx));
cx.subscribe(&screen, |_, event: &AgentInboxEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K moves between prompts, 1-3 picks options, A approves, D declines, L defers, R focuses reply, U undoes or reopens, and Left/Right switches Open/Done.

## API

Primary GPUI type: `AgentInboxScreen` (`kui::components::inbox::AgentInboxScreen`).
Event type: `AgentInboxEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
