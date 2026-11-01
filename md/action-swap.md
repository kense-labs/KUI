# Action swap

Follow a person, save an article, or send an invitation and recover from a simulated network failure.

`kui add action-swap`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::action_swap::ActionSwap;

let control = cx.new(|_| ActionSwap::new("follow", ["Follow", "Updating", "Following", "Retry"]));
use crate::ui::primitives::commit::Commit;
cx.subscribe(&control, |control, request: &Commit, cx| {
    // An in-memory success. Finish here only after the real operation returns.
    control.update(cx, |view, cx| view.complete(request.ticket, true, cx));
}).detach();
```

Mount the entity as a GPUI child. Keep selected state only after host acknowledgement. A failed invitation can be retried once the connection is restored.

Only changed letters move. Without a working label, the pending button narrows to a square with control-radius corners.

## Interface

The control uses the theme’s `radius.md` (8px by default).

- Event: `Commit { ticket, next }`.
- API: `complete(ticket, ok, cx)`.
- Implementation: `registry/src/components/action_swap.rs`.
- Preview: [English](/en/kui/components/action-swap/) · [中文](/kui/components/action-swap/).
