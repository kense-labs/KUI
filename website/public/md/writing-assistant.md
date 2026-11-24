# Writing assistant

Floating AI writing card with a soft drifting color glow, quick suggestion chips, word-by-word streaming replies, and live voice dictation.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add writing-assistant`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::writing_assistant::{WritingAssistant, WritingAssistantEvent};

let assistant = cx.new(|cx| {
    WritingAssistant::new("quill-helper", cx)
        .with_name("Quill")
        .with_tagline("Your writing companion.")
});
cx.subscribe(&assistant, |_, event: &WritingAssistantEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press Enter in the prompt field to send, Shift+Enter for a newline, Tab to step through suggestion chips, composer, voice dictation, and New chat, or click Accept, Retry, and Copy under any streamed draft.

## API

Primary GPUI type: `WritingAssistant` (`kui::components::writing_assistant::WritingAssistant`).
Event type: `WritingAssistantEvent`.

Configure persona copy with `with_name`, `with_tagline`, `with_greeting`, `with_placeholder`, custom chips with `with_suggestions`, custom generation with `with_responder`, and speech behavior with `with_speech_mode` (`SpeechMode::Simulate` or `SpeechMode::Hidden`). Supports light and dark `Theme` modes, bilingual `UiLanguage`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Three Gaussian-softened pink, violet, and blue lobes drift gently across the lower half and dim to `42%` once a thread starts. Dictation breathes an outward pulse ring with a three-bar live waveform while streaming words into the composer, and replies stream word by word. `Theme::reduced_motion` freezes the glow and shows completed replies immediately.

Desktop and web use the same GPUI source.
