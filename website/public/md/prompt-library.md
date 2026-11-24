# Prompt library

Pick saved AI prompt templates, type directly into inline `{variable}` chips inside the sentence, watch the assembled prompt preview update live below, and dispatch once every slot is filled.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add prompt-library`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::prompt_library::{PromptLibrary, PromptLibraryEvent};

let prompts = cx.new(|cx| {
    PromptLibrary::new("workspace-prompts", cx)
        .with_default_template("release")
});
cx.subscribe(&prompts, |_, event: &PromptLibraryEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Search or select saved templates in the sidebar, click any inline `{variable}` chip inside the prompt sentence to edit its value (`Enter` jumps to the next unfilled variable or triggers `Use prompt` when ready), watch filled values and missing `{holes}` update in the `PREVIEW` card, and click `Copy` or `Use prompt`.

## API

Primary GPUI type: `PromptLibrary` (`kui::components::prompt_library::PromptLibrary`).
Event type: `PromptLibraryEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_use_label(...)`, `with_templates(...)`, `with_default_template(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Switching templates glides the active pill along the sidebar via `pro_pill_indicator` while cross-transitioning the detail pane via `pro_view_transition`; edited variable slots briefly illuminate in both the sentence and preview, token estimates roll live via `pro_rolling_number`, and `Theme::reduced_motion` disables movement cleanly. Below `540px`, the template list stacks above the prompt editor.

Desktop and web use the same GPUI source.
