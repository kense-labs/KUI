# ExplanationNote

Disclose a reasoning summary beside the response it explains.

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
kui add reasoning-block
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::reasoning_block::{ExplanationNote};

let control = cx.new(|_| ExplanationNote::new("explanation",vec![("Read sources".into(),"Checked release notes and public docs.".into())]));
```

## Interaction

While streaming, the preview shows the newest two lines. Natural completion opens the full transcript for reading or copying. You can fold it again. Stop preserves the text received so far, and Retry starts a fresh stream. The live demo waits while the finished transcript is open.

## API

`set_state` · `(NoteState, cx)`

Pass progress headings and paragraphs as notes, or supply cumulative plain text to set_transcript. NoteState supports Live, Finished, Stopped and Interrupted. Supply elapsed seconds and an optional error. plain() embeds the view in a message, and set_expanded controls its disclosure. Subscribe to StopRequested to cancel the producer and RetryRequested to restart it.

Event: `StopRequested, RetryRequested`.

## Motion

The 44px header uses the light- or dark-surface Kense mark and stays in place. New chunks fade for 140ms. Wrapped preview lines glide upward on a 240ms spring. After two lines, the top 16px of the 40px preview fades the outgoing text into the surface. Measured disclosure height follows a reversible 260ms spring. Labels crossfade over 100/160ms, and the live label shimmers while seconds roll. Reduced motion renders the settled positions and omits shimmer and chunk fades.

Desktop and web use the same GPUI view.
