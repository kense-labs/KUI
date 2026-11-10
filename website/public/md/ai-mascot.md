# AI mascot

Represent assistant activity with a vector character, eyes and a companion dot.

## Install

```bash
kui add ai-mascot
```

Use `kui::` with the `kense-kui` crate and `crate::ui::` with copied source. Call `kui::init(cx)` before opening the application window.

## Usage

```rust
use gpui::prelude::*;
use kui::components::ai_mascot::{AiMascot, MascotStatus};

let control = cx.new(|_| {
    AiMascot::new("Kense")
        .with_status(MascotStatus::Thinking)
        .with_size(24.0)
        .follow_pointer(false)
});
```

## API

`set_status(MascotStatus, &mut Context<Self>)` changes the state. The initial state is `Idle`. The six states are `Idle`, `Listening`, `Thinking`, `Writing`, `Done` and `Error`.

`with_size(f32)` defaults to 96px and accepts 24–160px. Sizes below 56px use compact framing. `with_tone(MascotTone)` defaults to `Accent` and also accepts `Ink` and `Soft`. Use `with_color(Hsla)` and `with_eye_color(Hsla)` to override the body and eyes. `with_companion(bool)` defaults to `true`.

`with_pointer_follow(MascotPointerFollow)` accepts `None`, `Hover` and `Page`. The default is `Hover`. Touch and reduced motion suppress pointer tracking. The image exposes its name and current status to assistive technology.

## Motion

The host can pause with `set_paused` and suspend updates with `set_visible`. A state change blends from the current pose over 420ms. Reduced motion displays a static pose. The preview cycles the six poses until a state is picked manually.
