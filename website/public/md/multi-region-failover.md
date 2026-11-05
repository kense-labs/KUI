# Multi-region failover

Visualize host-provided regional health and traffic shifts.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add multi-region-failover`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::multi_region_failover::{FailoverSnapshot, MultiRegionFailover};

let snapshot = FailoverSnapshot::new("East", "West");
let control = cx.new(|_| MultiRegionFailover::new(snapshot));
```

## Interaction

The host owns health and requests. Only resolve a matching request ID; failure preserves the prior snapshot, and stale responses are ignored.

## API

`resolve` · `(u64, Result<FailoverSnapshot, SharedString>, &mut Context<Self>)`

Stage buttons request a phase; call resolve after the asynchronous operation, and set_visible to stop off-screen motion. The demo intentionally fails the first failover request to exercise retry.

Event: `FailoverRequest`.

## Motion

Confirmed weights interpolate, particles pause when hidden or reduced motion is enabled. The reference's detailed 2.5D scene is not yet reproduced, and visual, touch and screen-reader validation remain pending.

Desktop and web use the same GPUI view.
