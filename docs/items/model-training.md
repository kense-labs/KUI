# Model training

Explain the stages of an illustrative model training run.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add model-training`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::model_training::{ModelTraining, TrainingConfig};

let control = cx.new(|_| ModelTraining::new(TrainingConfig::default()));
```

## Interaction

Values are illustrative and must not be presented as actual training metrics. The host should synchronize visibility to stop animation off screen.

## API

`set_config` · `(TrainingConfig, &mut Context<Self>)`

Configure data counts, classes, epochs, loss and target through TrainingConfig. Use stage buttons or set_step to inspect and set_paused for playback.

Event: `TrainingStepChanged, TrainingPlaybackChanged`.

## Motion

A 21-second sequence shows signal travel, learning, loss and evaluation; reduced motion shows a still frame. The stacks and fine-grained motion differ from the reference and visual, touch and screen-reader validation remain pending.

Desktop and web use the same GPUI view.
