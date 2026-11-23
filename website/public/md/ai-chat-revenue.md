# AI chat revenue

Answer revenue questions in chat with KPI cards, an interactive 13-week YoY area chart with range brush follow-ups, regional splits, and Q4 forecast.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add ai-chat-revenue`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ai_chat_revenue::{AiChatRevenueScreen, AiChatRevenueEvent};

let screen = cx.new(|cx| AiChatRevenueScreen::new("ai-chat-revenue", cx));
cx.subscribe(&screen, |_, event: &AiChatRevenueEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Drag or Shift+Left/Right across weeks to brush-select a range and ask a scoped follow-up, toggle Weekly/Monthly capsule pills, and open Stripe/HubSpot sheets.

## API

Primary GPUI type: `AiChatRevenueScreen` (`kui::components::ai_chat_revenue::AiChatRevenueScreen`).
Event type: `AiChatRevenueEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
