# Provider status

Show host-provided provider health and incident history.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add provider-status`; replace `kui::` imports below with `crate::ui::`.

## Usage

Use `with_open_incident(id)` to show an incident from the initial snapshot. The provider ID must identify a row with an incident.

```rust
use gpui::prelude::*;
use kui::components::provider_status::{ProviderHealth, ProviderStatus, ProviderStatusItem};

let providers = vec![ProviderStatusItem::new("main", "Main provider", ProviderHealth::Operational)];
let control = cx.new(|_| ProviderStatus::new("AI providers", providers));
```

## Interaction

The host owns data, polling, visibility and opening incident URLs. The example uses simulated data.

## API

`set_providers` · `(Vec<ProviderStatusItem>, &mut Context<Self>)`

On Refresh capture current_request and return using set_providers_for or set_error_for; these ignore older and settled checks. Legacy unscoped setters require host-side stale-response filtering. The host drives set_remaining_seconds.

Event: `ProviderStatusEvent`.

## Motion

Incident details use a 240 ms spring and measured height; reduced motion shows the final state. Visual and touch verification are pending.

Desktop and web use the same GPUI view.
