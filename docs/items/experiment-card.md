# Experiment card

An A/B test card in two lines that quietly unfolds on hover—variants cross-linked to a miniature chart, a metric switcher with rolling numbers, statistical confidence, and one-click winner confirmation.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add experiment-card`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::experiment_card::{
    ExperimentCard, ExperimentCardEvent, ExperimentTone,
};

let card = cx.new(|cx| {
    ExperimentCard::new("signup-copy-test", cx)
        .with_title("Signup button copy")
        .with_status("Running", ExperimentTone::Green)
        .with_note("Day 9 of 14")
        .with_action_prefix("Ship")
});
cx.subscribe(&card, |_, ev: &ExperimentCardEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Hover or click the header to unfold the variant comparison, switch between Conversion, Revenue, and Retention (or press Left / Right), hover any variant to cross-highlight its head bar and inspect confidence/visitors, and click a variant row to pick and confirm a winner.

## API

Primary GPUI type: `ExperimentCard` (`kui::components::experiment_card::ExperimentCard`).
Event type: `ExperimentCardEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `lower_is_better` metrics, `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

Unfolds in place with `pro_fold`, slides metric pills via `pro_pill_indicator`, rolls every variant value and percentage lift via `pro_rolling_number`, and springs comparison bars via `pro_spring_f32`. Reduced motion switches values and heights instantaneously.

Desktop and web use the same GPUI source.
