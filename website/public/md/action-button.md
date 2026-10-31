# Action button

Save changes → Saving → Saved; the acknowledgement resets after 2.4 seconds.

`kui add action-button`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::action_button::ActionButton;

let control = cx.new(|_| ActionButton::new("save", ["Save changes", "Saving", "Saved", "Retry"]));
use crate::ui::primitives::commit::Commit;
cx.subscribe(&control, |control, request: &Commit, cx| {
    // An in-memory success. Finish here only after the real operation returns.
    control.update(cx, |view, cx| view.complete(request.ticket, true, cx));
}).detach();
```

Mount the entity as a GPUI child. Host completes the current ticket. Pending suppresses duplicate clicks; stale completions are ignored.

The label rises 2px and fades in over 160ms. The shell follows its shaped width; success resets after 2.4s.

## Interface

The control uses the theme’s `radius.md` (8px by default).

- Event: `Commit { ticket, next }`.
- API: `complete(ticket, ok, cx)`.
- Implementation: `registry/src/components/action_button.rs`.
- Preview: [English](/en/kui/components/action-button/) · [中文](/kui/components/action-button/).
