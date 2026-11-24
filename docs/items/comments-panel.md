# Comments panel

Collaborate across human and AI threads with sliding filter tabs, runnable `/edit` slash commands, `@mention` pills, resolve transitions, and an inline composer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add comments-panel`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::comments_panel::{CommentsPanel, CommentsPanelEvent};

let panel = cx.new(|cx| {
    CommentsPanel::new("doc-comments", cx)
        .with_current_user("Lena Hart")
});
cx.subscribe(&panel, |_, event: &CommentsPanelEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Switch between `Open`, `Resolved`, and `All` threads via the sliding pill bar, sort by recency or runnable `/edit` commands, click `Run edit` on any `/edit` comment or reply to apply and undo AI revisions, toggle resolution per thread, and compose replies or new comments with `@mention` and `/edit` chips.

## API

Primary GPUI type: `CommentsPanel` (`kui::components::comments_panel::CommentsPanel`).
Event type: `CommentsPanelEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Sliding filter pills (`pro_pill_indicator`), `/edit` execution shimmer (`pro_shimmer_text`), feed view transitions (`pro_view_transition`), and reply banner disclosures (`pro_fold`) respect `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
