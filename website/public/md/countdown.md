# Countdown

Show the remaining time until a deadline.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add countdown`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::countdown::Countdown;
use std::time::Duration;

let control = cx.new(|cx| Countdown::new("Launch", Duration::from_secs(95), cx).with_complete_label("Live now"));
```

## Interaction

The display updates until zero, then shows the completion label.

## API

`reset` · `(Duration, &mut Context<Self>)`

Supply a duration and an optional completion label. CountdownFinished fires once; reset starts a new run.

Event: `CountdownFinished`.

## Motion

Digital reels roll to each new value with a 0.26s spring; reduced motion snaps. The completion transition is currently not sampled at render time, so the finished state switches without a fade.

Desktop and web use the same GPUI view.
