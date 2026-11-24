# Fallback chain

Model failover flow diagram connecting an incoming `Request` node across smooth cubic bezier curves to an ordered stack of primary and fallback LLM providers (`Claude`, `GPT`, `Gemini`, `DeepSeek`) and onward into `Response`—showing red broken branches on HTTP 429/529 failures, a traveling light pulse on the active attempt, and interactive toggles to simulate outages on any model.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add fallback-chain`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::fallback_chain::{FallbackChain, FallbackChainEvent};

let chain = cx.new(|cx| {
    FallbackChain::new("fallback-chain", cx)
        .with_title("Summarize support ticket")
        .with_route("kense/auto")
});
cx.subscribe(&chain, |_, event: &FallbackChainEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click `Healthy` / `Fail 529` on any prioritized model row to toggle whether that provider succeeds or fails and immediately re-run the failover wave. Click `Request`, any model card, or `Response` to inspect the incoming prompt, provider/region metadata, or streamed answer below. Press `Replay` (or `Retry` when every model fails) to re-send the request.

## API

Primary GPUI type: `FallbackChain` (`kui::components::fallback_chain::FallbackChain`).
Event type: `FallbackChainEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_route(...)`, `with_request(...)`, `with_response(...)`, `with_attempts(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

A glowing pulse runs along the incoming cubic curve to whichever model is currently `Trying`; failed curves split open in red with a crisp break cross; and once a model answers, its outgoing curve lights emerald green with a pulse into `Response`. Respects `Theme::reduced_motion` and adapts cleanly below `640px` and `540px`.

Desktop and web use the same GPUI source.
