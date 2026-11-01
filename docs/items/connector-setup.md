# ConnectionWizard

Connect an external service and track the setup state.

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
kui add connector-setup
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::connector_setup::{ConnectionWizard, Service, ConnectionState};

let control = cx.new(|_| ConnectionWizard::new("tools",vec![Service{id:"git".into(),name:"Git".into(),description:"Search commits".into(),scopes:vec!["read".into()],state:ConnectionState::Offline}]));
```

## Interaction

Report authorization and sync progress using the active ticket; disconnect requires confirmation.

## API

`progress` · `(service, ticket, ConnectionState, cx)`

Service carries granted scopes and ConnectionState. Return progress with its service ID and original request ticket.

Event: `ConnectionAction`.

## Motion

Scopes and disconnect confirmation expand on a 240ms spring. The 2px sync line follows real progress samples over 160ms.

Desktop and web use the same GPUI view.
