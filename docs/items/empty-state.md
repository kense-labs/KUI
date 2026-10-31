# EmptyState

Explain an empty view and offer a next action.

`kui add empty-state`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::empty_state::{EmptyState, EmptyContent};

let control = cx.new(|_| EmptyState::new("saved", EmptyContent::new("No saved views", "Save a view to reuse your filters.").with_action("Create view")));
```

Keep the entity mounted and call set_content to change its message. Subscribe to EmptyAction to handle its optional button.

## Interaction

Tab to the optional action. Enter or Space activates it. The old copy rises out while the new copy and icon enter. The body height follows its content.

- Event: `EmptyAction`.
- Preview: [English](/en/kui/components/empty-state/) · [中文](/kui/components/empty-state/).
