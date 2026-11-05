# Voice recorder

Voice recorder with a host-controlled audio adapter.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add voice-recorder`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::voice_recorder::VoiceRecorder;

let control = cx.new(|_| VoiceRecorder::new("Voice note"));
```

## Interaction

The host provides the device or audio playback adapter.

## API

`set_phase` · `(RecorderPhase, &mut Context<Self>)`

On Start, Stop, Send or Retry capture request_id; use set_phase_for, set_error_for and append_message_for with that ID. Capture session_id for set_audio_for and set_recording_error_for. Message IDs deduplicate confirmations. Legacy unscoped methods remain available but the host must filter stale callbacks itself.

Event: `RecorderEvent`.

## Motion

The visual follows state and respects reduced motion.

Desktop and web use the same GPUI view.
