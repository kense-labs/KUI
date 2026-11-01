# GenerationReadout

Inspect token usage and timing for a generated response.

## Install

```bash
kui add generation-stats
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::generation_stats::{GenerationReadout, Metrics};

let control = cx.new(|_| GenerationReadout::new("stats","Assistant",Metrics {first_token_ms:400,duration_ms:2400,input_tokens:800,output_tokens:120,input_rate:2.,output_rate:8.,..Metrics::default()}));
```

## Interaction

Compute speed after the first token and cost using the host’s per-million prices.

## API

`set_metrics` · `(Metrics, cx)`

Metrics accepts first-token and total milliseconds, input/output counts, rates per million and throughput samples.

## Motion

The details use measured height and include parent spacing in the same spring. Output token drops snap to a new run; increases interpolate over 180ms.

Desktop and web use the same GPUI view.
