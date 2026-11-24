# AI answer

Stream grounded assistant answers with horizontal source cards, inline citation previews, version switching, feedback tags, and related follow-ups.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add ai-answer`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ai_answer::{AiAnswer, AiAnswerEvent};

let answer = cx.new(|cx| {
    AiAnswer::new("offsite-ai-answer", cx)
        .with_question("Where should we hold the team offsite in Lisbon?")
        .with_framed(true)
});
cx.subscribe(&answer, |_, event: &AiAnswerEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover or focus any inline `[1]` citation chip to highlight its horizontal source card and open a live source excerpt popover; hover a source card to light up every citation referencing it. Use `Copy` to write clean prose plus a numbered bibliography to the clipboard, `Regenerate` or `< 1/2 >` to switch generated versions, thumbs up/down to record feedback with unfolding reason tags, `Replay stream` / `Stop` to control token streaming, and follow-up rows to explore related questions.

## API

Primary GPUI type: `AiAnswer` (`kui::components::ai_answer::AiAnswer`).
Event type: `AiAnswerEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation (`640px` down to narrow two-card layouts below `480px`).

## Motion

Status header shimmer (`pro_shimmer_text`), word-by-word token streaming with pulsing accent caret, citation popover presence (`pro_presence`), version micro-glides (`pro_view_transition`), and thumbs-down reason disclosure (`pro_fold`) follow KUI curves and respect `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
