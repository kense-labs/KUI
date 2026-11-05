# LLM cost calculator

Compare estimated token costs using rates and usage provided by your application.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add llm-cost-calculator`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::llm_cost_calculator::{LlmCostCalculator, LlmModelPrice, LlmUsage};

let prices = vec![LlmModelPrice::new("sample", "Sample model", "Example", 1.0, 3.0)];
let usage = LlmUsage::new(8000.0, 500.0, 5000.0);
let control = cx.new(|cx| LlmCostCalculator::new(prices, usage, cx).with_simulated(true));
```

## Interaction

All prices and usage are supplied by the host. Prices are USD per million tokens; the estimate is not a bill. Negative, non-finite or overflowing values return no estimate. Sample prices are explicitly labelled.

## API

`set_usage` · `(LlmUsage, &mut Context<Self>)`

set_models updates rates; set_usage updates workload; the in-panel reset button restores the initial workload (`reset` is internal to the view, not part of the host API). Subscribe to LlmCostEvent for usage and selection changes. compute_llm_cost is a pure function returning Option<LlmCost>. Four sliders edit workload and cache hit rate; host values outside slider ranges still calculate exactly until edited.

Event: `LlmCostEvent`.

## Motion

Ranks and bars currently update immediately. Reordering springs, rolling totals, loading and error integration, and real desktop, Web, touch and screen-reader acceptance remain pending.

Desktop and web use the same GPUI view.
