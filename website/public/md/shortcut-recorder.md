# ShortcutRecorder

Capture and review a keyboard shortcut. The field shows the current combination as separate platform keycaps. The recorder emits one change after an accepted edit.

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
kui add shortcut-recorder
```

For copied source, replace `kui::` imports with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::shortcut_recorder::ShortcutRecorder;

let control = cx.new(|cx| {
    let mut recorder = ShortcutRecorder::new("Open command menu", Some("Mod+K".into()), cx);
    recorder.set_bindings(vec![("Alt+N".into(), "Create issue".into())]);
    recorder
});
```

## Interaction

Click the field or focus it and press Enter or Space to record. Hold a modifier to preview its keycap. A letter requires Command, Control, or Alt/Option. F1–F24 can be recorded alone. Escape, leaving the field, or using Tab to move focus cancels recording. Backspace and Delete clear the binding. The reset action restores the initial value.

An occupied or system-reserved combination enters review. Cancel preserves the committed value. Use anyway commits the proposed combination and returns focus to the field. The host owns the binding table and can clear the previous owner after confirmation.

## API

- `value() -> Option<&str>` returns the committed value, including while reviewing a conflict.
- `set_value(value, cx)` replaces the controlled value silently. `ShortcutChanged(Option<String>)` reports an accepted edit or clear.
- `set_bindings(Vec<(String, String)>)` supplies occupied combinations with their human-readable action names.
- `set_platform(ShortcutPlatform, cx)` selects the presentation and matching convention. By default, KUI detects the host. The stored `Mod` modifier maps to Command on macOS and Control on Windows.
- `ShortcutPlatform::spoken` supplies readable names for symbolic keys. The preview uses the same keycaps in the recorder and searchable shortcut list.

## Motion

While recording, the indicator breathes over 1.4 seconds. New keycaps fade and rise into place, with a small stagger for completed combinations. Removed keycaps fade over 100 ms. A held keycap lowers one pixel. The conflict message and its actions expand below the field with a shared spring. With reduced motion, the indicator is steady and keycaps settle immediately.
