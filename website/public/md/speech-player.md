# Speech player

Control host-provided speech with synchronized text.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add speech-player`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::speech_player::SpeechPlayer;

let control = cx.new(|_| SpeechPlayer::new("Read aloud", "A thoughtful interface."));
```

## Interaction

The host performs synthesis and playback; the component sends requests and shows host-confirmed state.

## API

`with_model(model)` attributes the text to a model. `with_transport(duration)` shows the playback controls before a host session begins.

`set_progress` · `(Option<usize>, f32, f32, &mut Context<Self>)`

Capture session_id on Start or Retry and use set_phase_for, set_progress_for and set_error_for for asynchronous results. Stop ends the session, so late results from it are ignored; the host still cancels its synthesis or playback. Unscoped methods remain compatible but require host-side filtering.

Event: `SpeechEvent`.

## Motion

The word highlight travels on a line and jumps between lines; reduced motion skips movement.

Desktop and web use the same GPUI view.
