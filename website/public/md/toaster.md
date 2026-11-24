# Toaster

Calm stacked notification toasts with trigger controls for `Default`, `Success`, `Action / Undo`, `Error`, and `Promise (Saving -> Saved)`, hover fan-out, paused timers, and drag-to-swipe dismissal.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add toaster`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::toaster::{Toaster, ToasterEvent, ToasterPosition};

let toaster = cx.new(|cx| {
    Toaster::new("app-toaster", cx)
        .with_position(ToasterPosition::BottomRight)
        .with_visible_toasts(3)
});
cx.subscribe(&toaster, |_, event: &ToasterEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Hover the notification stack or press `F8` to fan stacked cards out vertically and pause auto-dismiss timers. Click `Undo` on action toasts, press `Esc` to dismiss the front toast, or drag any card horizontally to swipe-dismiss (short drags rubber-band back).

## API

Primary GPUI type: `Toaster` (`kui::components::toaster::Toaster`).
Event type: `ToasterEvent`.

Push programmatic notifications with `push_toast(ToastItem)`, `ToastKind::Default`, `ToastKind::Success`, `ToastKind::Action`, `ToastKind::Error`, and `ToastItem::with_promise_resolution(ms, title, desc)`. Configure stack behavior with `with_visible_toasts`, `with_duration`, `with_expand`, `with_hairline`, and `with_triggers`.

## Motion

Stacked cards spring their vertical `top` and width `scale` smoothly on hover fan-out. Promise notifications morph in place from a rotating `28%` arc into a full ring while progressively stroking a check mark and crossfading copy. `Theme::reduced_motion` snaps fan-out and card transitions immediately.

Desktop and web use the same GPUI source.
