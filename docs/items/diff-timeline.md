# Diff timeline

Visualize what a coding agent changed in a pull request edit by edit—cumulative lines `+added` and `−removed` as dual overlaid gradient curves, per-file segmented diff bars, and interactive monospace diff previews.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add diff-timeline`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::diff_timeline::{DiffTimeline, DiffTimelineAgent, DiffTimelineEvent};

let timeline = cx.new(|cx| {
    DiffTimeline::new("pr-214-diff", cx)
        .with_agent(DiffTimelineAgent::new("frontend", "Frontend agent"))
        .with_context(Some("PR #214".into()))
        .with_live(true)
});
cx.subscribe(&timeline, |_, ev: &DiffTimelineEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Move the pointer across the chart, click any segmented file-bar piece, or focus the chart and press `ArrowLeft` / `ArrowRight` / `Home` / `End` (`Escape` to dismiss) to inspect each edit's timestamp, `+added` / `−removed` counts, note, and syntax-tinted diff lines. Hovering a file row pins its edits as colored markers along the `0` baseline. Click `Replay` in the legend footer to watch the run unfold from `0:00`.

## API

Primary GPUI type: `DiffTimeline` (`kui::components::diff_timeline::DiffTimeline`).
Supporting types: `DiffEdit`, `DiffSnippetLine`, `DiffLineKind`, `DiffTimelineAgent`.
Event type: `DiffTimelineEvent` (`InspectEdit(Option<usize>)`, `ReplayStarted`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_max_rows(...)`, `with_loading(true)` skeletons, and `with_empty(true)` empty states.

## Motion

While `live`, the 3x3 agent pixel mark twinkles, the `+added` and `−removed` curve endpoints breathe soft concentric rings, per-file bars widen via `Slide` springs, and a subtle wash glides across the active file row; `Theme::reduced_motion` disables continuous motion cleanly.

Desktop and web use the same GPUI source.
