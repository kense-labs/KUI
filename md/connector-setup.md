# ConnectionWizard

Connect an external service and track the setup state.

## Install

```bash
kui add connector-setup
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::connector_setup::{ConnectionWizard, Service, ConnectionState};

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
