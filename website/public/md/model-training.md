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

A 21-second sequence shows signal travel, learning, loss and evaluation. Signals travel along the curved links between layers. Training and held-out data sit beside the network on wider layouts and above it on narrow layouts. Loss and epoch appear below the network. Reduced motion shows a still frame.

Desktop and web use the same GPUI view.
