# Model usage

AI provider usage breakdown—one stacked bar per company shaded from darkest top model to lighter sibling models, a Tokens / Cost / Requests switcher, hover model inspection, and dynamic task-category filtering.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add model-usage`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::model_usage::{ModelUsage, ModelUsageEvent, UsageMetric};

let usage = cx.new(|cx| {
    ModelUsage::new("weekly-model-usage", cx)
        .with_title("This week")
        .with_default_metric(UsageMetric::Tokens)
});
cx.subscribe(&usage, |_, ev: &ModelUsageEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Switch between Tokens, Cost, and Requests at the top right, hover or press Left / Right / Home / End across AI company bars (Anthropic, OpenAI, Google, DeepSeek) to inspect per-model shares in a side card, and watch the Search / Analyze / Code / Test task breakdown filter to that provider.

## API

Primary GPUI type: `ModelUsage` (`kui::components::model_usage::ModelUsage`).
Event type: `ModelUsageEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Stacked provider columns rise sequentially on mount, morph heights and model-segment shares smoothly via `pro_spring_f32` when switching measures, glide the measure tab via `pro_pill_indicator`, and roll figures via `pro_rolling_number`. Reduced motion snaps all transitions.

Desktop and web use the same GPUI source.
