# GenerationReadout

Inspect token usage and timing for a generated response.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add generation-stats
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::generation_stats::{GenerationReadout, Metrics};

let control = cx.new(|_| GenerationReadout::new("stats","Assistant",Metrics {first_token_ms:400,duration_ms:2400,input_tokens:800,output_tokens:120,input_rate:2.,output_rate:8.,..Metrics::default()}));
```

## Interaction

Read first-token latency, throughput, output tokens and cost. Open Details for latency and per-token cost, and copy the stats as JSON. Failed runs can be retried.


## API

`set_metrics` · `(Metrics, cx)`

Use Metrics for counts, latency, prices and samples. set_state provides the five run states, set_usage adds cached and reasoning tokens, and variant selects a card or inline readout. RetryGeneration is emitted on Retry.

Event: `RetryGeneration`.


## Motion

Details unfolds over 260ms; output counts interpolate over 100ms and snap down on a reset. The sparkline keeps up to 48 samples.

Desktop and web use the same GPUI view.
