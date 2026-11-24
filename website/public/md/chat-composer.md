# Chat composer

Compose rich AI prompts with a multiline editor, provider-branded model popover, live voice waveform dictation, connected app switches, file chips, and inline tool approval.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add chat-composer`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::chat_composer::{ChatComposer, ChatComposerEvent};

let composer = cx.new(|cx| {
    ChatComposer::new("assistant-composer", cx)
        .with_placeholder("Ask anything, attach files, or run connected tools…")
});
cx.subscribe(&composer, |_, event: &ChatComposerEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Type multiline prompts (`Enter` or `Cmd+Enter` to send, `Shift+Enter` for a newline), attach or remove file chips with `+`, review and `Allow` / `Deny` inline tool requests above the write card, pick between `Claude`, `GPT`, `Gemini`, and `DeepSeek` models, toggle `GitHub`, `Linear`, `Slack`, `Notion`, and `Google Drive` connections, or click the microphone to record with a live audio waveform and progressive word transcript.

## API

Primary GPUI type: `ChatComposer` (`kui::components::chat_composer::ChatComposer`).
Event type: `ChatComposerEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Popover presence (`pro_presence`), inline tool-approval and attachment disclosures (`pro_fold`), capsule app switches (`pro_switch`), approval elevation shimmer (`pro_shimmer_text`), and 14-bar voice dictation waveforms respect `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
