# AI chat spend

Break down monthly AI model spend in chat with stacked provider columns, interactive routing/caching savings sliders, and 6-month forecast.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add ai-chat-spend`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ai_chat_spend::{AiChatSpendScreen, AiChatSpendEvent};

let screen = cx.new(|cx| AiChatSpendScreen::new("ai-chat-spend", cx));
cx.subscribe(&screen, |_, event: &AiChatSpendEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click provider bars to ask model-specific questions, drag 5%-step savings levers to recompute monthly savings live, and press / to focus the composer.

## API

Primary GPUI type: `AiChatSpendScreen` (`kui::components::ai_chat_spend::AiChatSpendScreen`).
Event type: `AiChatSpendEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
