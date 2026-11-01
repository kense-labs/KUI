# Theme switcher

Switch the document theme. Compare Reveal, Eclipse, Split and Rise.

`kui add theme-switch`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::theme_switch::ThemeSwitch;

let control = cx.new(|_| ThemeSwitch::new("appearance"));
use crate::ui::components::theme_switch::ThemeChange;
cx.subscribe(&control, |_control, request: &ThemeChange, cx| {
    crate::ui::foundation::Theme::global_mut(cx).set_mode(request.next);
    cx.refresh_windows();
}).detach();
```

The website begins the visual change on primary pointer-down. Releasing outside the control or cancelling the gesture restores the previous theme. Keyboard activation commits once.

Mount the entity as a GPUI child. The host receives the next theme and trigger bounds in window coordinates. Skip the page animation when reduced motion is selected.

Reveal wipes from the left (380ms), Eclipse expands from the center (430ms), Split opens at the center seam (390ms), and Rise moves up from the bottom (360ms).

## Interface

The control uses the theme’s `radius.md` (8px by default).

- Event: `ThemeChange { next, mode, trigger }`.
- API: `with_mode(Transition)`.
- Implementation: `registry/src/components/theme_switch.rs`.
- Preview: [English](/en/kui/components/theme-switch/) · [中文](/kui/components/theme-switch/).
