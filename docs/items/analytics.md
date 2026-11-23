# Analytics

Explore revenue KPIs, multi-channel time-series curves with hover scrubbing, zebra sparkline breakdowns, top products, sessions, and AOV comparisons.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add analytics`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::analytics::{AnalyticsScreen, AnalyticsEvent};

let screen = cx.new(|cx| AnalyticsScreen::new("analytics", cx));
cx.subscribe(&screen, |_, event: &AnalyticsEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press 1-4 to switch 24h/7d/30d/90d windows, click any channel legend or product row to filter the entire dashboard, and clear active filter pills anytime.

## API

Primary GPUI type: `AnalyticsScreen` (`kui::components::analytics::AnalyticsScreen`).
Event type: `AnalyticsEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
