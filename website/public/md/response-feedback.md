# Response feedback

Collect a rating, structured reasons and an optional note for a response.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add response-feedback`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::response_feedback::ResponseFeedback;

let control = cx.new(|cx| ResponseFeedback::new("feedback", "message-1", window, cx));
```

## Interaction

The host sends Submit events and reports the matching request ID through complete; failure preserves the draft for retry. The example simulates failure and recovery.

## API

`complete` · `(u64, bool, &mut Context<Self>)`

Choose a rating, toggle reasons, add an optional note and submit. Duplicate pending submissions and stale callbacks are ignored.

Event: `ResponseFeedbackEvent`.

## Motion

The inline panel expands and folds with a spring and follows the reduced-motion preference. Fine-grained transitions and focus handoff still require visual and accessibility verification.

Desktop and web use the same GPUI view.
