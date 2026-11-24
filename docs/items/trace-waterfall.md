# Trace waterfall

Debug multi-agent execution on a live millisecond timeline—hierarchical span bars for orchestrators, sub-agents, LLM calls, and tools with slowest-call detection, folded child marks, and inline span inspectors.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add trace-waterfall`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::trace_waterfall::{
    TraceWaterfall, TraceWaterfallEvent,
};

let trace = cx.new(|cx| {
    TraceWaterfall::new("agent-trace-waterfall", cx)
        .with_mark_slowest(true)
});
cx.subscribe(&trace, |_, ev: &TraceWaterfallEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Click any span row (or press Up/Down and Enter/Space) to unfold its inline inspector (`pro_fold`) showing exact start/end timestamps, duration, cost, tokens, prompt/input, response/output, and error diagnostics. Click a parent branch chevron (or press Left/Right) to fold its child spans into compact colored marks right inside the parent bar.

## API

Primary GPUI type: `TraceWaterfall` (`kui::components::trace_waterfall::TraceWaterfall`).
Event type: `TraceWaterfallEvent`.

Supports `TraceSpanKind` (`Agent`, `SubAgent`, `Llm`, `Tool`), `TraceSpanStatus` (`Running`, `Done`, `Failed`), `with_now_ms`, `with_min_range_ms`, `with_mark_slowest`, light/dark `Theme`, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Running spans grow along the moving `"now"` cursor with a sweeping highlight sheen, time rescaling springs smoothly via `pro_spring_f32`, folded child spans condense into inline marks on the parent bar, and span inspectors unfold via `pro_fold`. Respects `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
