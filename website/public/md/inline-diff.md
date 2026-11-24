# Inline diff

Review AI prose edits inline inside a natural paragraph—struck red deletions beside green insertions that you accept or reject one by one with keys, a gliding popover, `Accept all` / `Reject all`, and `Undo`.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add inline-diff`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::inline_diff::{InlineDiff, InlineDiffEvent};

let review = cx.new(|cx| {
    InlineDiff::new("prose-inline-diff", cx)
        .with_label(Some("Suggested by AI · Make it shorter".into()))
});
cx.subscribe(&review, |_, event: &InlineDiffEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover or click any change—or press `Tab`, `Shift+Tab`, `←`, or `→`—to move the floating `Accept` / `Reject` popover between pending edits. Press `Enter` (or `Y`) to accept, `Backspace` (or `N`) to reject, `⌘Z` (or `Z`) to undo individual or bulk actions, and `Accept all` / `Reject all` in the bottom bar to resolve every remaining suggestion.

## API

Primary GPUI type: `InlineDiff` (`kui::components::inline_diff::InlineDiff`).
Event type: `InlineDiffEvent`.
Helper function: `diff_words(original, suggested) -> Vec<DiffSegment>`.

Supports light/dark `Theme` modes, bilingual `UiLanguage`, loading skeletons, empty states, and compact toolbar adaptation below `400px`.

## Motion

Accepting shrinks struck deletions away while the green tint fades into normal prose; rejecting collapses green insertions while original words settle back. The review popover glides across changes via `pro_spring_f32` and the completion checkmark draws itself smoothly; `Theme::reduced_motion` resolves immediately.

Desktop and web use the same GPUI source.
