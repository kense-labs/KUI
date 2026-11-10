# Eclipse theme transition

Open the new theme with an expanding ellipse.

## Install

```bash
kui add theme-switch-eclipse
```

Use `kui::` imports with the crate or `crate::ui::` imports with copied source. Initialize KUI with `kui::init(cx)` before opening the window.

## Usage

```rust
use gpui::prelude::*;
use kui::components::theme_switch::{ThemeChange, ThemeSwitch, Transition};

let control = cx.new(|_| ThemeSwitch::new("appearance").with_mode(Transition::Eclipse));
cx.subscribe(&control, |_, request: &ThemeChange, cx| {
    kui::foundation::Theme::global_mut(cx).set_mode(request.next);
    cx.refresh_windows();
}).detach();
```

The subscription above applies the theme. Animate the containing surface in the host when needed. `ThemeChange` carries `next`, `mode` and `trigger` bounds in window coordinates.

## Interaction and motion

The sun and moon use the same center throughout the transition. The accessible name defaults to the next theme. Override it with `with_accessible_label`. Press Enter or Space to activate. The KUI website uses a 430ms document transition. Reduced motion applies the theme immediately.
