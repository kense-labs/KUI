# SetupChecklist

Track independent setup tasks and their instructions.

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
kui add onboarding-checklist
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::onboarding_checklist::{SetupChecklist, ChecklistItem};

let control = cx.new(|_| SetupChecklist::new("Set up workspace", vec![ChecklistItem::new("Invite team", "Invite a collaborator.", "Send invite")]));
```

## Interaction

Expand a task to read instructions. Its separate completion control updates the count.

## API

`new` · `(heading, Vec<ChecklistItem>)`

Expand a task to read instructions. Its separate completion control updates the count.

Event: `TaskChanged`.

## Motion

Task details, the completed group, and the whole list fold through their measured heights. A checked task stays in place for 720ms before joining the completed group.

Desktop and web use the same GPUI view.
