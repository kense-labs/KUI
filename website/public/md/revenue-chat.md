# Revenue chat

Embed a standalone conversational revenue analyst with KPI cards, pointer-brushable YoY trend charts, regional stacked-bar splits, Q4 forecast cones, and a prompt composer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add revenue-chat`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::revenue_chat::{RevenueChatBlock, RevenueChatEvent};

let block = cx.new(|cx| RevenueChatBlock::new("revenue-chat", cx));
cx.subscribe(&block, |_, event: &RevenueChatEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Drag horizontally across any line or forecast chart (or press Left/Right and Enter) to brush a week range and ask a follow-up scoped to that stretch. Click a stacked bar to ask about that region, press `/` to focus the composer, and press Escape to dismiss an active selection.

## API

Primary GPUI type: `RevenueChatBlock` (`kui::components::revenue_chat::RevenueChatBlock`).
Event type: `RevenueChatEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation without requiring `AppShell`.

## Motion

Assistant pixel marks twinkle and status text shimmers while reading Stripe and HubSpot, brushed selection bands and tooltips glide smoothly, and streaming replies respect Theme::reduced_motion.

Desktop and web use the same GPUI source.
