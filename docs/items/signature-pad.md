# Signature pad

Draw and edit a signature, then export its strokes as SVG.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add signature-pad`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::signature_pad::SignaturePad;

let control = cx.new(|cx| SignaturePad::new("Sign here", cx));
```

## Interaction

Draw with a mouse or touch; undo, redo, replay, export PNG or clear. With the canvas focused, use Cmd/Ctrl+Z, Shift+Cmd/Ctrl+Z and Delete. The demo shows export status; the host supplies file saving.

## API

`to_svg` · `(u32, u32) -> String`

strokes returns normalized points and pressure. to_svg and to_png produce SVG text and PNG bytes; saving the bytes is the host’s responsibility. GPUI mouse and touch input uses neutral pressure; host pen adapters can provide real pressure with begin_sample and append_sample.

Event: `SignatureChanged / SignaturePngExported`.

## Motion

Ink follows input as pressure-scaled strokes. Replay reveals samples in order; reduced motion shows the complete signature immediately.

Desktop and web use the same GPUI view.
