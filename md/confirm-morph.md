# Confirm morph

Delete a sample project, cancel, retry or undo without opening a separate dialog.

`kui add confirm-morph`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::confirm_morph::ConfirmMorph;

let control = cx.new(|cx| ConfirmMorph::new("remove", "Delete project", "Delete project?", window, cx));
use crate::ui::components::confirm_morph::ConfirmRequest;
cx.subscribe(&control, |control, request: &ConfirmRequest, cx| {
    // An in-memory success. Finish here only after the real operation returns.
    control.update(cx, |view, cx| view.complete(request.ticket, true, cx));
}).detach();
```

Mount the entity as a GPUI child. Click arms the action. Hold requires one second on the answer; Countdown gives a cancellable three-second grace period after the answer.

## Interface

The control, its nested actions and their highlights use the theme’s `radius.md` (8px by default).

- Event: `ConfirmRequest { ticket, intent }`.
- API: `set_mode(ConfirmMode, cx) / complete(ticket, ok, cx)`.
- Implementation: `registry/src/components/confirm_morph.rs`.
- Preview: [English](/en/kui/components/confirm-morph/) · [中文](/kui/components/confirm-morph/).
