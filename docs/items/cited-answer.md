# Cited answer

Inspect RAG answers wired two-way to their sources—hovering a citation or source spotlights both ends, while clicking unfolds the quoted passage with matching phrases marked.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add cited-answer`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::cited_answer::{CitedAnswer, CitedAnswerEvent};

let answer = cx.new(|cx| {
    CitedAnswer::new("rag-cited-answer", cx)
        .with_pages_read(12)
});
cx.subscribe(&answer, |_, event: &CitedAnswerEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover or focus any inline `[1]` marker or sentence segment to tint that claim and spotlight its source card; hover or focus a source row to highlight every sentence backed by that source while dimming unrelated prose. Click either a citation marker or a source row to unfold (`pro_fold`) its verbatim passage with matching phrases highlighted, copy the quote, or open the source.

## API

Primary GPUI type: `CitedAnswer` (`kui::components::cited_answer::CitedAnswer`).
Event type: `CitedAnswerEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Measured-height quote disclosures (`pro_fold`) and footer status crossfades (`pro_view_transition`) glide smoothly and snap immediately when `Theme::reduced_motion` is active.

Desktop and web use the same GPUI source.
