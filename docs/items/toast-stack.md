# Toast stack

Show a host-controlled collection of transient notifications.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add toast-stack`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::toast_stack::ToastStack;

let control = cx.new(|_| ToastStack::new("Notifications"));
```

## Interaction

Push, update or dismiss notifications by ID. Hover pauses each timer.

## API

`push` · `(message, action, Duration, &mut Context<Self>) -> ToastId`

The queue holds 12 and shows 3 by default; hovering or focusing expands the stack and pauses its timers.

Event: `StackToastEvent`.

## Motion

Notifications tuck behind the front toast and fan out with a spring; reduced motion changes immediately.

Desktop and web use the same GPUI view.
