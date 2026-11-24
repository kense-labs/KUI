# Reasoning summary

Summarize how an agent spent its thinking time on a single answer — featuring a twinkling `pro_agent_avatar`, live reasoning clock, multi-segment cognitive bar (`Search`, `Read`, `Plan`, `Verify`), cross-linked hover highlighting, clickable model effort/token/cost disclosures, and collapsible `pro_fold` source and step traces.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add reasoning-summary`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::reasoning_summary::{
    EffortLevel, ReasonKind, ReasonModel, ReasonStep, ReasoningSummary, ReasoningSummaryEvent,
};

let summary = cx.new(|cx| {
    ReasoningSummary::new("answer-reasoning", cx)
        .with_steps(vec![
            ReasonStep::new("s1", ReasonKind::Search, "Searched API docs for 429 limits", 1_800).with_model("opus"),
            ReasonStep::new("s2", ReasonKind::Read, "Compared token bucket headers", 2_100).with_model("grok"),
            ReasonStep::new("s3", ReasonKind::Plan, "Designed adaptive backoff policy", 2_300).with_model("gpt"),
            ReasonStep::new("s4", ReasonKind::Verify, "Verifying against sandbox burst", 1_900).with_model("gpt").with_running(true),
        ])
        .with_models(vec![
            ReasonModel::new("opus", "Claude Opus 4.6", "claude", "Anthropic").with_effort(EffortLevel::Medium).with_cost(0.086),
            ReasonModel::new("grok", "Grok 3", "xai", "xAI").with_effort(EffortLevel::High).with_cost(0.064),
            ReasonModel::new("gpt", "GPT-5", "openai", "OpenAI").with_effort(EffortLevel::Max).with_cost(0.142),
        ])
});
cx.subscribe(&summary, |_, ev: &ReasoningSummaryEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hovering any bar segment, cognitive kind pill, or model row highlights all associated segments, kinds, and models while softly fading unrelated items. Clicking a model row opens its `pro_fold` disclosure showing provider, effort, input/output tokens, USD cost, and router rationale. Clicking `Searched N sources` or `N steps` in the footer smoothly unfolds the searched sources or step trace list.

## API

Primary GPUI type: `ReasoningSummary` (`kui::components::reasoning_summary::ReasoningSummary`).
Supporting types: `ReasonStep`, `ReasonKind`, `ReasonModel`, `EffortLevel`, `ReasonSource`.
Event type: `ReasoningSummaryEvent` (`ModelToggled`, `SourcesToggled`, `StepsToggled`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

While reasoning is active, `pro_agent_avatar` twinkles in a diagonal wave, the active bar segment grows with a sweeping specular shine, and `pro_fold` discloses model telemetry, sources, and step traces smoothly; `Theme::reduced_motion` disables continuous shimmers and twinkles.

Desktop and web use the same GPUI source.
