# Dropzone

Upload multiple documents and assets with a fanning three-sheet drop target, colored file-type badges, live progress with remaining time, and error recovery.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add dropzone`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::dropzone::{Dropzone, DropzoneEvent};

let uploader = cx.new(|cx| {
    Dropzone::new("attachments-dropzone", cx)
        .with_accept([".pdf", ".csv", ".png", ".json"])
        .with_max_size(25 * 1024 * 1024)
});
cx.subscribe(&uploader, |_, event: &DropzoneEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover or preview-drag over the dashed drop target to fan the stacked document sheets apart, click the dropzone (or `Enter` / `Space`) or quick file-type chips (`+ PDF`, `+ CSV`, `+ PNG`, `+ JSON`) to enqueue uploads, click `Retry` on failed transfers, or remove individual rows and clear the queue.

## API

Primary GPUI type: `Dropzone` (`kui::components::dropzone::Dropzone`).
Event type: `DropzoneEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_accept(...)`, `with_max_size(...)`, `with_hint(...)`, `with_items(...)`, `with_autoplay(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Hovering or dragging drives a critically-damped `Slide` spring that fans the left `PDF` and right `PNG` document sheets outward while lifting the central upload sheet; per-row progress bars interpolate via `pro_spring_f32`, active transfers sweep a subtle highlight with `pro_shimmer_text`, failed rows play a brief damped shake, and `Theme::reduced_motion` completes changes instantaneously.

Desktop and web use the same GPUI source.
