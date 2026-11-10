# Action morph

Open a quick-create menu from a floating action button. Each action grows the same surface into a small form and retains its own draft while the control is mounted.

## Installation

```sh
kui add action-morph
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::action_morph::{
    ActionMorph, ActionMorphAction, ActionMorphComposer, ActionMorphSubmit,
};
use kui::primitives::glyph::Glyph;

let create = cx.new(|cx| {
    ActionMorph::new(
        vec![ActionMorphAction::new(
            "task",
            "Task",
            Glyph::Check,
            ActionMorphComposer::new("New task", "What needs doing?", "Add task", "Task added"),
        ).with_shortcut('t')],
        window,
        cx,
    )
    .with_deferred_submit()
    .with_style(|e| e.absolute().bottom(gpui::px(16.)).right(gpui::px(16.)))
});
window.subscribe(&create, cx, |control, submission: &ActionMorphSubmit, w, cx| {
    // This example accepts immediately. Resolve with the persistence result in an app.
    control.update(cx, |control, cx| {
        control.complete(submission.ticket, Ok(()), w, cx);
    });
}).detach();
```

Mount the view in a positioned GPUI container. Use `ActionMorphCorner::Start` with a start-side position to expand toward the opposite side. The control leaves placement to the host.

## API

| Input | Default | Behavior |
| --- | --- | --- |
| `new(actions, window, cx)` | Required | Actions have stable IDs, labels, glyphs, and composer configurations. |
| `with_label(label)` | `"Create"` in English | Trigger name and menu title. |
| `with_corner(ActionMorphCorner)` | `End` | Logical bottom anchor. |
| `with_reset_after(Duration)` | 1600ms | Check dwell before the plus returns. |
| `with_style(Fn(Div) -> Div)` | None | Apply host positioning or layout. |
| `with_deferred_submit()` | Off | Hold the saving state until `complete(ticket, result, window, cx)`. |
| `set_max_width(Pixels, cx)` | Window minus 32px | Constrain an embedded surface. |
| `ActionMorphSubmit` | — | Carries ticket, action ID, trimmed text, and optional choice value. |

Use `with_choices` for an optional single-selection group. `with_default_choice` selects the given value or falls back to the first choice. `with_multiline(true)` keeps Enter as a newline and uses Cmd/Ctrl+Enter to submit. In a single-line editor Enter submits directly.

## Interaction and motion

- The 56px circular trigger opens a menu up to 256px wide. The composer grows up to 320px, with the same bottom corner fixed throughout. Menu and composer rectangles use the shared 8px KUI radius.
- The menu supports arrow keys, Home, End, and an action's configured shortcut. Escape returns from the composer to the menu, then to the trigger. Click outside to dismiss while idle.
- The current and outgoing faces crossfade and travel in opposite directions. One spring per surface edge keeps the shape continuous on interruptions. Reduced motion settles the shape immediately and keeps a short face fade.
- Empty input stays in the composer. A failed save retains the input and selection. Only the outstanding ticket can resolve a save. Success clears that action's draft and returns focus to the trigger.
