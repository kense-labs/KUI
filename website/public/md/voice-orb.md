# Voice orb

Voice orb with a host-controlled audio adapter.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add voice-orb`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::voice_orb::VoiceOrb;

let control = cx.new(|cx| VoiceOrb::new(cx));
```

## Interaction

The host provides the device or audio playback adapter.

## API

`set_state` · `(VoiceState, &mut Context<Self>)`

Subscribe to requests and provide actual state from the host.

Event: `VoiceOrbEvent`.

## Motion

The visual follows state and respects reduced motion.

Desktop and web use the same GPUI view.
