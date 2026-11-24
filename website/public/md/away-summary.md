# Away summary

Catch up on what an agent finished, failed on, or needs from you right where you left off in a chat thread, and fold it into a quiet one-line divider with `"Got it"`.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add away-summary`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::away_summary::{AwaySummary, AwaySummaryEvent};

let summary = cx.new(|cx| {
    AwaySummary::new("thread-away-summary", cx)
        .with_since_duration("2h 14m")
        .with_limit(5)
});
cx.subscribe(&summary, |_, event: &AwaySummaryEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Updates are grouped automatically with `Needs you` first, then `Failed`, then `Done`, each carrying one direct action (`Approve`, `Retry`, `View`). Resolving an item updates it in place so rows below never jump. Click `Show N more` to expand overflow updates, click `Got it` (or press `Escape`) to fold the card into a quiet one-line thread divider, and click the divider pill at any time to reopen it.

## API

Primary GPUI type: `AwaySummary` (`kui::components::away_summary::AwaySummary`).
Event type: `AwaySummaryEvent`.

Supports `AwayItemKind` (`NeedsYou`, `Failed`, `Done`), `AwayTimeStyle` (`Duration`, `Clock`), optional surrounding thread chrome (`with_thread_chrome(bool)`), light/dark `Theme` modes, and bilingual `UiLanguage`.

## Motion

`Needs you` target markers breathe gently while awaiting input, `pro_fold` glides smoothly between the expanded card and the one-line thread divider (as well as overflow rows), and `Theme::reduced_motion` freezes continuous pulses.

Desktop and web use the same GPUI source.
