# Image reveal

Show host-controlled image generation progress.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add image-reveal`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::image_reveal::ImageReveal;

let control = cx.new(|_| ImageReveal::new("Quiet room", 1.5));
```

## Interaction

The host begins each run with a unique ID, optionally supplies a preview, and reports progress, completion or failure. The example is simulated.

## API

`set_progress` · `(u64, f32, &mut Context<Self>)`

begin, set_preview, set_progress, finish and fail ignore replies from older or finished runs. Retry is emitted to the host.

Event: `ImageRevealEvent`.

## Motion

Progress follows updates over 160 ms and reveals the preview from top to bottom. Image blur, grain and soft-edge effects are not yet implemented or visually verified.

Desktop and web use the same GPUI view.
