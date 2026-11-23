# Assistant home

Start a new AI session from a centered greeting, multi-mode prompt composer, folder and app connectors, and starter chips.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add assistant-home`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::assistant_home::{AssistantHomeScreen, AssistantHomeEvent};

let screen = cx.new(|cx| AssistantHomeScreen::new("assistant-home", cx));
cx.subscribe(&screen, |_, event: &AssistantHomeEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press / to focus the composer, 1-7 to launch starter prompts, toggle connected apps with capsule switches, and submit to transition into live chat.

## API

Primary GPUI type: `AssistantHomeScreen` (`kui::components::assistant_home::AssistantHomeScreen`).
Event type: `AssistantHomeEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
