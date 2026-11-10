# Orbit menu

Offer a short arc of actions on a single object. Tap to keep the menu open or press, drag and release to run an action in one gesture.

## Install

```bash
kui init
kui add orbit-menu
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::orbit_menu::{OrbitAction, OrbitIcon, OrbitInvoked, OrbitMenu, OrbitTone};
use kui::primitives::glyph::Glyph;

let menu = cx.new(|cx| {
    OrbitMenu::new(
        vec![
            OrbitAction::new("pin", "Pin", Glyph::Pin).with_done("Pinned"),
            OrbitAction::new("share", "Share", OrbitIcon::Share),
            OrbitAction::new("remove", "Remove", Glyph::Trash).with_tone(OrbitTone::Danger),
        ],
        window,
        cx,
    )
    .with_label("Note actions")
});
cx.subscribe(&menu, |_, event: &OrbitInvoked, _| {
    // Dispatch event.0 to the host's action handler.
    let action = &event.0;
    debug_assert!(matches!(action.as_ref(), "pin" | "share" | "remove"));
}).detach();
```

Use `crate::ui` as the import root when installing source. The host owns each action's effect and any confirmation required by that effect.

| API | Default and behavior |
| --- | --- |
| `OrbitAction` | Stable ID, label and icon. Confirmation defaults to the label. `OrbitTone` defaults to `Neutral`. Use a `Glyph`, a built-in mark, or a monochrome `Svg` asset. |
| `with_label`, `with_id` | Localized “Actions” label and a generated root ID. |
| `with_radius`, `with_spread`, `with_direction` | 104px radius, 150° spread and −90° upward. Direction increases clockwise. Place the trigger so the arc has room to open. |
| `with_disabled`, `set_disabled` | `false` by default. Disabling closes the arc and prevents input. |
| `with_announce` | `true` by default. Announces the short confirmation as status. |
| `set_actions` | Update the host's actions and retain identity by ID. Use three to six actions for a readable arc. |
| `OrbitInvoked`, `OrbitOpenChanged` | Reports the chosen action ID and transitions into or out of the interactive arc. |

Press the trigger and drag past the 34px dead zone toward a choice. Release to invoke it once. A release off the arc, a cancelled gesture or a stationary hold dismisses the menu. A brief tap leaves the arc open for a separate click. Click outside or tap the trigger again to close it.

Arrow keys open the arc and select an action for keyboard focus. Up starts at the last action. Within the arc, the arrow keys wrap and Home or End jumps to an edge. Enter or Space invokes the focused action. Escape closes and restores the trigger's focus. Tab closes and continues to the next control.

The confirmation lasts 1.4 seconds. Reduced motion settles the arc and skips the swirl and magnetic movement. On Web, reserve drag ownership for the trigger when placing the menu inside a scrollable surface.
