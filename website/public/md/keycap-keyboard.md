# Keycap keyboard

Display a mechanical keyboard that lights and moves as a person types, clicks, or taps. A multiline readout shows the newest text above the board.

## Installation

```sh
kui add keycap-keyboard
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::keycap_keyboard::{
    KeycapKeyboard, KeycapLayout, KeycapListen, KeycapValueChanged,
};

let control = cx.new(|cx| {
    KeycapKeyboard::new(cx)
        .with_layout(KeycapLayout::Compact, cx)
        .with_listen(KeycapListen::Focus)
        .with_placeholder("Type anything")
});
window.subscribe(&control, cx, |_, _: &KeycapValueChanged, _, _| {}).detach();
```

## API

| Builder or method | Default | Behavior |
| --- | --- | --- |
| `with_layout(KeycapLayout, &App)`, `set_layout(KeycapLayout, &mut Context<Self>)` | `Compact` | The compact board has five rows. `Letters` has three letter rows, Shift, Delete and Space. Switching layouts preserves text. |
| `with_rows(Vec<Vec<Keycap>>, &App)` | Preset rows | Overrides the layout. Use `Keycap::new` or `Keycap::from` with a character or the token `space`, `backspace`, `enter`, `shift`, `tab`, `caps`, `control`, `alt` or `platform`. `Keycap::with_span` sets its width in key units. |
| `with_default_value(impl Into<SharedString>)`, `value()` | Empty | Sets the initial text and reads the retained text. |
| `with_value(impl Into<SharedString>)`, `set_value(impl Into<SharedString>, &mut Context<Self>)` | Uncontrolled | Controlled input emits a proposed value. The host accepts it by calling `set_value`. |
| `with_max_length(usize)` | 280 | Retains the newest graphemes. Backspace removes one grapheme. |
| `with_listen(KeycapListen)`, `with_window_filter(Fn(&Window, &App) -> bool)` | `Focus` | Focus listens inside the board. Window additionally observes unconsumed keys in the same unfocused GPUI window. A host filter can allow other known non-text controls. Command, Control and Alt shortcuts pass through. |
| `with_placeholder(impl Into<SharedString>)`, `with_show_output(bool)` | Localized “Start typing”, `true` | Sets the empty readout text or hides the readout. |
| `with_label(impl Into<SharedString>)`, `with_id(impl Into<ElementId>)`, `with_style(Fn(Div) -> Div)` | Localized “Keyboard”, `keycap-keyboard`, no style override | Sets the accessible name, stable identity and optional board styling. |
| `with_visibility(Fn(&App) -> bool)` | Window and layout visibility | Optionally suspend animation and observation when an embedding host hides the preview. |
| `KeycapValueChanged(pub SharedString)` | No initial event | Emits a changed value after a physical or on-screen edit. |

## Interaction and motion

- Focus the readout or a key and type. Click or tap a cap to activate it on press. A focused cap also activates on Enter or Space. On-screen Shift latches until a printable cap consumes it. Tab still moves focus.
- Keycaps use one measured unit for width, travel, gaps, radii and legends. The widest row defines the unit, up to 54px.
- Press travel reaches about 92% in 35ms. Release returns in 200ms with a slight lift, and the light decays for 700ms. New graphemes fade over 140ms. Caret blinking resumes after idle. Reduced motion keeps the light response with stationary keycaps and caret.
