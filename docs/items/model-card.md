# ModelSheet

Inspect a model’s capabilities and select it for a task.

## Install

```bash
kui add model-card
```

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::model_card::{ModelSheet, Model};

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
