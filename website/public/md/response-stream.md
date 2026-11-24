# Response stream

Render one streamed agent answer across every state it passes through—words arriving smoothly, stopped and continued from the exact word index, failed mid-sentence and retried from that point, and completed—with steer and queued message chips displayed cleanly above the answer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add response-stream`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::response_stream::{ResponseStream, ResponseStreamEvent, ResponseStreamStatus};

let stream = cx.new(|cx| {
    ResponseStream::new("atlas-answer", cx)
        .with_agent("atlas", "Atlas")
        .with_status(ResponseStreamStatus::Streaming)
        .with_meta(182, 6_400)
});
cx.subscribe(&stream, |_, event: &ResponseStreamEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press `Esc` (or `Stop`) while streaming to pause on the exact word index, then press `Continue` to resume without repeating earlier text. Switch states in the interactive bar (or call `fail_stream` / `retry_from_failure`) to test mid-sentence failure recovery, inject a steer chip (`Inject steer`) to watch an inline bend divider land in the answer, or `Copy` and `Regenerate` once `Done`.

## API

Primary GPUI type: `ResponseStream` (`kui::components::response_stream::ResponseStream`).
Event type: `ResponseStreamEvent`.

Pass `.with_controls(false)` when embedding the standalone answer without the interactive simulation bar. Supports fenced code blocks, light/dark `Theme` modes, bilingual `UiLanguage`, loading skeletons, and empty states.

## Motion

Arriving words fade in over `180ms` beside a breathing blue caret and diagonal pixel-mark wave; `Theme::reduced_motion` renders steady text and caret immediately.

Desktop and web use the same GPUI source.
