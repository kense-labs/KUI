# EmptyState

Explain an empty view and offer a next action.

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
kui add empty-state
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::empty_state::{EmptyState, EmptyContent};

let control = cx.new(|_| EmptyState::new("saved", EmptyContent::new("No saved views", "Save a view to reuse your filters.").with_action("Create view")));
```

Keep the entity mounted and call set_content to change its message. Subscribe to EmptyAction to handle its optional button.

## Interaction

Tab to the optional action. Enter or Space activates it. The old copy rises out while the new copy and icon enter. The body height follows its content.

- Event: `EmptyAction`.
- Preview: [English](/en/kui/components/empty-state/) · [中文](/kui/components/empty-state/).
