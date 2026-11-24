# Prompt explorer

Explore what people ask AI assistants with a drifting atmospheric hero composer, live search completions, prompt volume analysis, and popular and trending keyword lists.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add prompt-explorer`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::prompt_explorer::{PromptExplorerEvent, PromptExplorerScreen};

let screen = cx.new(|cx| PromptExplorerScreen::new("prompt-explorer", cx));
cx.subscribe(&screen, |_, event: &PromptExplorerEvent, _| {
    let _ = event;
})
.detach();
```

## Interaction

Use Up/Down in the combobox field to move through relevant searches, Enter to pick a completion or analyse your prompt (or Cmd+Enter to analyse every line when Bulk analysis is enabled), Escape to close suggestions or clear analysis results, Left/Right to switch between Prompts/Page and Rising/Falling, and `/` to focus the prompt composer.

## API

Primary GPUI type: `PromptExplorerScreen` (`kui::components::prompt_explorer::PromptExplorerScreen`).
Event type: `PromptExplorerEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, `with_bulk_analysis(true)` multi-line batch prompt analysis, and responsive desktop-to-mobile adaptation.

## Motion

Three incommensurate blue, cyan, and lavender light layers drift smoothly across the hero backdrop while fading into the page canvas before the keyword card begins. Completion popovers, rising analysis rows, spring share meters, and skeleton sweeps respect `Theme::reduced_motion`.

Desktop and web use the same GPUI source.
