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

Open a call to inspect its input and output, and copy the input. Approve, deny or retry one call at a time; a running call rejects duplicates.


## API

`new` · `(id, Vec<Invocation>)`

Invocation binds its named tool, arguments, output and ToolState. Use durations for optional timing labels. Resolve actions by the index carried in ToolAction.

Event: `ToolAction`.


## Motion

The group and detail panels fold to measured height over 260ms; rows enter with a 20ms stagger. Reduced motion keeps labels and status icons steady.

Desktop and web use the same GPUI view.
