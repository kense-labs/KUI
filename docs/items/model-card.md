# ModelSheet

Inspect a model’s capabilities and select it for a task.

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
kui add model-card
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::model_card::{ModelSheet, Model};

let control = cx.new(|_| ModelSheet::new("model",vec![Model{name:"Atlas".into(),provider:"Kense".into(),description:"Workspace assistant".into(),features:vec!["Text".into()],context:128000,max_output:32000,input_rate:2.,output_rate:8.,scores:vec![("Code".into(),0.82)]}]));
```

## Interaction

Compare model limits and prices using data from the host. Sample estimates use 80% input.

## API

`choose` · `(index, cx)`

Model provides limits, input/output prices and normalized benchmark scores. ModelChanged reports the selected index.

Event: `ModelChanged`.

## Motion

The model name and description crossfade, while limits, input cost and normalized benchmark bars interpolate. The monthly volume slider updates the estimate synchronously.

Desktop and web use the same GPUI view.
