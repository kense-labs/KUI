# Action button

Save changes → Saving → Saved; the acknowledgement resets after 2.4 seconds.

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
kui add action-button
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::action_button::ActionButton;

let control = cx.new(|_| ActionButton::new("save", ["Save changes", "Saving", "Saved", "Retry"]));
use kui::primitives::commit::Commit;
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
