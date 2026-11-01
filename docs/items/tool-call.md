# InvocationInspector

Inspect a tool invocation, its arguments and its result.

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
kui add tool-call
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tool_call::{InvocationInspector, Invocation, ToolState};

let control = cx.new(|_| InvocationInspector::new("calls",vec![Invocation { name:"Search".into(),action:"Find release issues".into(),input:"release".into(),output:String::new(), state:ToolState::Approval }]));
```

## Interaction

Approval, denial and retry target one invocation. A pending invocation rejects duplicate actions.

Tool names share a measured column. The invocation title and its status share a left edge. Deny and Allow sit on the right on wide panels. Below 440px they move underneath the status and align with the title. Retry uses the same placement.

## API

`new` · `(id, Vec<Invocation>)`

Invocation binds a tool name and description to input/output strings and ToolState. Use the ToolAction index to resolve approvals or retries.

Event: `ToolAction`.

## Motion

The group and each call's input/output disclosure fold to measured height on a 240ms spring. Tool status transitions independently of the card.

Desktop and web use the same GPUI view.
