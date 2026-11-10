# Tone tuner

Tune an editable draft with host-provided rewrites.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add tone-tuner`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::tone_tuner::ToneTuner;

let control = cx.new(|cx| ToneTuner::new("Reply", "Thanks for your message.", window, cx));
```

## Interaction

The host runs rewrites and handles cancellation. The sample is simulated; Copy uses the real clipboard.

## API

`finish` · `(u64, impl Into<String>, &Window, &mut Context<Self>) -> bool`

A settings change waits 220 ms, then emits Rewrite with a ticket. Stop or a new edit emits Cancel; finish and fail reject stale results or a changed draft. User edits become the source for future rewrites.

Use `with_recipient(name, subject)` to add the optional recipient header. The Changes switch toggles the highlights without requesting a rewrite.

Event: `ToneTunerEvent`.

## Motion

Sliders use shared KUI motion and changed words can be highlighted. Sentence crossfades and full reference comparison remain unimplemented or unverified.

Desktop and web use the same GPUI view.
