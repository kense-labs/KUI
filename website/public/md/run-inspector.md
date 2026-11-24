# Run inspector

Inspect an entire multi-agent run on a single screen with top rollup metrics, a live orchestrator-to-sub-agent tree, miniature call duration bars, and inline foldable error traces.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add run-inspector`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::run_inspector::{RunInspectorBlock, RunInspectorEvent};

let block = cx.new(|cx| RunInspectorBlock::new("run-inspector", cx));
cx.subscribe(&block, |_, event: &RunInspectorEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click or press Enter on any agent card to scope the top summary strip to that agent's subtree; click again or press Escape to return to the whole run. Click any miniature call row to unfold its arguments, output, or red error trace and `Retry` action right beneath that row.

## API

Primary GPUI type: `RunInspectorBlock` (`kui::components::run_inspector::RunInspectorBlock`).
Event type: `RunInspectorEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), optional side-by-side `supervisor` spans, `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile tree adaptation.

## Motion

Active tree connectors carry smooth light pulses, running call bars display a sweeping sheen, and inline call disclosures fold smoothly while respecting Theme::reduced_motion.

Desktop and web use the same GPUI source.
