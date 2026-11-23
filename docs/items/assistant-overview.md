# Assistant overview

Combine a centered prompt composer with a 4-column bento overview featuring a 280-day activity heatmap, recent chats, credits, and generated files.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add assistant-overview`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::assistant_overview::{AssistantOverviewScreen, AssistantOverviewEvent};

let screen = cx.new(|cx| AssistantOverviewScreen::new("assistant-overview", cx));
cx.subscribe(&screen, |_, event: &AssistantOverviewEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Arrow keys navigate the 40x7 activity heatmap, Home/End jumps to first or last day, / focuses the composer, and file rows open the workspace panel.

## API

Primary GPUI type: `AssistantOverviewScreen` (`kui::components::assistant_overview::AssistantOverviewScreen`).
Event type: `AssistantOverviewEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
