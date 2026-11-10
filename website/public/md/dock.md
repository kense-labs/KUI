# Dock

Switch editor tools in a floating toolbar with integrated group trays.

## Install

```bash
kui init
kui add dock
```

Use `kui::` for the crate API and `crate::ui::` for copied source. Mount the entity after calling `kui::init(cx)`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::dock::{Dock, DockIcon, DockItem, DockSelected};

let items = vec![
    DockItem::new("move", "Move", DockIcon::Pointer).with_shortcut("V"),
    DockItem::new("shapes", "Shapes", DockIcon::Rectangle).with_items(vec![
        DockItem::new("rectangle", "Rectangle", DockIcon::Rectangle).with_shortcut("R"),
        DockItem::new("ellipse", "Ellipse", DockIcon::Ellipse).with_shortcut("O"),
    ]),
];
let control = cx.new(|cx| Dock::new(items, cx).with_value("move", cx));
cx.subscribe(&control, |control, event: &DockSelected, cx| {
    control.update(cx, |dock, cx| dock.set_value(Some(event.0.clone()), cx));
}).detach();
```

| API | Default or behavior |
| --- | --- |
| `DockItem` | Use stable unique IDs, a label and a `DockIcon`. Optional shortcut labels and badges are announced with the tool. Set direct group members with `with_items`. |
| `with_value`, `set_value` | Selected tool or group member. The initial value is absent. A group remembers its last shown member. |
| `DockSelected` | Emits the selected ID. The host applies its editor action and updates the value. |
| `with_reorderable`, `DockReordered` | Off by default. A drag or Alt+Arrow requests a new order with the moved ID and destination index. The host supplies the accepted order with `set_items`. |
| `with_label` | Accessible toolbar name. Defaults to Tools or its localized label. |

The toolbar uses fixed 40px slots and a 4px gap. Group trays use 36px members and rise above the bar. Leave space above it for the tray and label.

Arrow Left and Right move focus within the toolbar or tray. Home and End jump to the ends. Enter or Space selects the focused tool. Arrow Up opens a group's tray, and Arrow Down or Escape closes it and returns focus. Tab continues past an open tray. Alt+Arrow moves the focused tool when reordering is enabled.

Mouse hover displays the shared label after a short delay. Keyboard focus shows it immediately. Horizontal drags move tools while vertical touch gestures stay available to the surrounding page. Shortcut hints describe host commands. The host binds the shortcuts.

Selection, reorder and the tray retarget from their current positions. Reduced motion settles their positions immediately. Desktop and Web render the same GPUI control.
