# Memory panel

Grouped and editable assistant memory manager with a master `Use memory` capsule switch, `pro_pill_indicator` category filter chips, inline search, per-row `Editor` rename, `Pause`, and `Forget` actions, `+ New memory` composer, and a timed bottom `Undo` tray.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add memory-panel`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::memory_panel::{MemoryPanel, MemoryPanelEvent};

let panel = cx.new(|cx| {
    MemoryPanel::new("memory-panel", cx)
        .with_title("Memory")
});
cx.subscribe(&panel, |_, event: &MemoryPanelEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Flip `Use memory` in the top-right to disable personalization without deleting stored facts. Switch between `All`, `Preferences`, `Work`, `Projects`, and `People` filter chips or click the magnifier to search inline. Hover or focus any memory row to Edit its text inline (`Enter` or `⌘↵` to save, `Esc` to cancel), Pause/Resume it, or Forget it—forgetting reveals an `Undo` button in the bottom tray that restores the memory to its original slot.

## API

Primary GPUI type: `MemoryPanel` (`kui::components::memory_panel::MemoryPanel`).
Event type: `MemoryPanelEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_enabled(...)`, `with_memories(...)`, `with_categories(...)`, `with_max_height(...)`, `with_undo_seconds(...)`, `with_loading(true)`, and `with_empty(true)`.

## Motion

Category filter selection glides across chips via `pro_pill_indicator`; flipping `Use memory` off unfolds an informational banner via `pro_fold`; header and footer counts micro-roll via `pro_rolling_number`; and forgetting a row animates the `Undo` tray via `pro_presence`. Respects `Theme::reduced_motion` and adapts chip counts below `340px`.

Desktop and web use the same GPUI source.
