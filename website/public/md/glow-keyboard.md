# Glow keyboard

Render a self-typing AI chat input floating above a frosted glass keyboard where every keystroke sends a glowing color wave up a gradient floor.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add glow-keyboard`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::glow_keyboard::{GlowKeyboard, GlowKeyboardEvent};

let hero = cx.new(|cx| {
    GlowKeyboard::new("hero-glow-kb", cx)
        .with_placeholder("Ask AI anything...")
});
cx.subscribe(&hero, |_, ev: &GlowKeyboardEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Types through three prompts with uneven human cadence, holding `Shift` (`⇧`) for capitals, correcting a typo with `⌫`, and submitting with `↩`. Clicking the input or pressing any key switches immediately into interactive user typing: every clicked or typed key depresses its drawn glass keycap, flashes the composer halo at the matching horizontal spot, and sends a color wave rising out of the floor. Pressing `Escape` on an empty field resumes the demo.

## API

Primary GPUI type: `GlowKeyboard` (`kui::components::glow_keyboard::GlowKeyboard`).
Event type: `GlowKeyboardEvent` (`Submit(SharedString)`, `Change(SharedString)`, `DemoPaused(bool)`).

Supports light and dark `Theme` modes, custom prompt sequences (`with_prompts`), custom placeholder and stage background (`with_placeholder`, `with_background`), bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and `11 : 6` responsive container scaling.

## Motion

Two separable-Gaussian-blurred gradient layers—an orange-magenta-blue crest and a `plus-lighter` light crest—drift harmonically while localized Gaussian wave plumes surge upward from each depressed keycap; `Theme::reduced_motion` freezes floor motion and demo auto-typing while keeping manual typing responsive.

Desktop and web use the same GPUI source.
