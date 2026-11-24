# Thinking state

Display an AI assistant's live working trail with a ticking `0:08` timer, shimmering header, branded search source pills, progressive reasoning steps, and collapsible disclosure.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add thinking-state`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::thinking_state::{ThinkingState, ThinkingStateEvent};

let thinking = cx.new(|cx| {
    ThinkingState::new("assistant-thinking", cx)
        .with_done_label("Finished thinking")
        .with_step_ms(1600)
});
cx.subscribe(&thinking, |_, event: &ThinkingStateEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click the header to fold or expand the reasoning trail (`pro_fold`), hover or focus searched brand pills (`Notion`, `Slack`, `GitHub`, `Linear`) to preview source URLs, click `+3 more` to reveal additional workspace sources, and toggle between `Done` and `Replay` to inspect either completed or live-streaming states.

## API

Primary GPUI type: `ThinkingState` (`kui::components::thinking_state::ThinkingState`).
Event type: `ThinkingStateEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Diagonal pixel-mark waves, header and active-step shimmer (`pro_shimmer_text`), `6px` vertical step arrival glides, URL tooltip presence (`pro_presence`), and measured-height collapse (`pro_fold`) respect `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
