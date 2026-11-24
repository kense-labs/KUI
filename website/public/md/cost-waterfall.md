# Cost waterfall

Inspect where every cent and token of an agent run went—stepped waterfall bars across LLM calls, tool calls, and sub-agents with model artwork, most-expensive highlight, and a `Cost` / `Tokens` switch.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add cost-waterfall`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::cost_waterfall::{
    CostMeasureMode, CostWaterfall, CostWaterfallEvent,
};

let waterfall = cx.new(|cx| {
    CostWaterfall::new("run-cost-waterfall", cx)
        .with_task("Report on LLM API prices in 2026")
        .with_mode(CostMeasureMode::Cost)
});
cx.subscribe(&waterfall, |_, ev: &CostWaterfallEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Toggle `Cost` and `Tokens` in the top switch to glide every waterfall bar between dollar spend and token volume. Click `Most expensive` in the summary strip to unfold and inspect the costliest step, fold or unfold sub-agent rows with their chevrons (or Left/Right arrow keys), and click any row (or press Enter/Space) to open its Prompt/Response or Input/Output inspector.

## API

Primary GPUI type: `CostWaterfall` (`kui::components::cost_waterfall::CostWaterfall`).
Event type: `CostWaterfallEvent`.

Supports `CostStepKind` (`Agent`, `Llm`, `Tool`), `CostStepStatus` (`Running`, `Done`, `Failed`), negative prompt-cache credit adjustments, `with_max_height`, light/dark `Theme`, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Switching between `Cost` and `Tokens` glides every stepped bar (`pro_spring_f32`) while the segmented pill tracks via `pro_pill_indicator`; running steps sweep a soft highlight sheen and step inspectors unfold smoothly via `pro_fold`. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
