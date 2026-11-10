# Prompt cache

Explain host-provided prompt cache outcomes and request costs.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add prompt-cache`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::prompt_cache::{CacheRequest, CacheSegment, CacheSnapshot, PromptCache};

let segments = vec![CacheSegment::new("system", "System", 2400, gpui::rgb(0xa08ad2).into())];
let requests = vec![CacheRequest::new("one", "Request 1", 200)];
let control = cx.new(|_| PromptCache::new(segments, CacheSnapshot::new(requests)));
```

## Interaction

The host supplies outcomes, token counts, costs, latency and cache lifetime. Requests require matching IDs; failures preserve prior results and stale responses are ignored. Simulated values must be labelled.

## API

`resolve` · `(u64, Result<(CacheResult, CacheState, u64, Option<u64>), SharedString>, &mut Context<Self>)`

Next request emits PromptCacheAction; resolve supplies the result or error. An optional changed segment can be armed before sending.

Event: `PromptCacheAction`.

## Motion

Confirmed result bars transition into view. The local sample supports play, pause, step and reset, with a simulated expiry countdown. For a host-driven component, playback is optional via `with_sample_playback(autoplay)`; call `advance_sample(cx)` on visible 250ms ticks. The cache-flight animation and full visual comparison are pending.

Desktop and web use the same GPUI view.
