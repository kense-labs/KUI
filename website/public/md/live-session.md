# Live session

Work alongside a team of agents in real time with a live conversation stream, steer composer, step-by-step plan, and shared context window.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add live-session`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::live_session::{LiveSessionBlock, LiveSessionEvent};

let session = cx.new(|cx| {
    LiveSessionBlock::new("pricing-live-session", cx)
        .with_task("Launch the new pricing page")
        .with_live(true)
});
cx.subscribe(&session, |_, event: &LiveSessionEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Select any agent chip at the top (`Orchestrator`, `Researcher`, `Writer`, or `You`) to fade unrelated turns in the conversation stream and unrelated steps in the right-hand plan simultaneously; press `Escape` or `Everyone` to restore the full view. Send a steer note or `@mention` from the bottom composer, click any shared file chip to inspect its contents, or click `Compact` to reclaim context window tokens.

## API

Primary GPUI type: `LiveSessionBlock` (`kui::components::live_session::LiveSessionBlock`).
Event type: `LiveSessionEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation (`894px` down to `390px`).

## Motion

Twinkling 3x3 pixel avatars (`pro_agent_avatar`), sliding filter pills (`pro_pill_indicator`), active-step text shimmer (`pro_shimmer_text`), and rolling context token counts (`pro_rolling_number`) follow KUI motion curves and respect `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
