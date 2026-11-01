# SetupChecklist

Track independent setup tasks and their instructions.

## Install

```bash
kui add onboarding-checklist
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::onboarding_checklist::{SetupChecklist, ChecklistItem};

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
