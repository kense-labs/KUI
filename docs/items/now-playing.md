# Now playing

Now playing with a host-controlled audio adapter.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add now-playing`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::now_playing::{NowPlaying, Track};

let tracks = vec![Track::new("one", "Morning light", "Kense", 212., "https://images.unsplash.com/photo-1500530855697-b586d89ba3ee")];
let control = cx.new(|_| NowPlaying::new("Player", tracks));
```

## Interaction

The host provides the device or audio playback adapter.

## API

`set_playing` · `(bool, &mut Context<Self>)`

Subscribe to requests and provide actual state from the host.

Event: `PlayerEvent`.

## Motion

The visual follows state and respects reduced motion.

Desktop and web use the same GPUI view.
