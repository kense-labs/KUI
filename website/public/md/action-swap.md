# Action swap

Follow a person, save an article, or send an invitation and recover from a simulated network failure.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Library examples use `kui::` imports. Call `kui::init(cx)` before opening windows.
See the [platform guide](https://github.com/kense-labs/KUI/blob/main/docs/PLATFORMS.md)
for features and host setup. Never enable `test-support` on wasm32.

To own this component's source instead:

```bash
cargo install --git https://github.com/kense-labs/KUI --locked kui-cli
kui init
kui add action-swap
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::action_swap::ActionSwap;

let control = cx.new(|_| ActionSwap::new("follow", ["Follow", "Updating", "Following", "Retry"]));
use kui::primitives::commit::Commit;
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
