# Image generation

A host-driven four-image generation surface.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add image-generation`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::image_generation::ImageGeneration;

let control = cx.new(|cx| ImageGeneration::new(window, cx));
```

## Interaction

The host performs generation, upscale and download. Respond to each ticket independently and cancel work when asked. The preview uses local simulated data. Selecting a suggestion fills the prompt so it can be edited before generation.

## API

`finish` · `(u64, GeneratedImage, &mut Context<Self>)`

`with_suggestions` · `(Vec<SharedString>) -> Self`. Defaults to an empty list.

Call set_progress, finish or fail with the ticket. Stale results after Stop are ignored. A failed or stopped variation keeps its original, and a failure can be retried or restored. Each completed image uses its own reported dimensions. Cmd/Ctrl+Enter starts a batch; plain Enter inserts a newline.

Event: `ImageGenerationEvent`.

## Motion

Progress follows host updates with a 240 ms transition. The pixel mosaic, upscale sweep and full visual comparison are not yet implemented or verified.

Desktop and web use the same GPUI view.
