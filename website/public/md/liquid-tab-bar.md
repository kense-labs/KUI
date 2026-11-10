# Liquid tab bar

Switch app sections with a sliding selection that inverts the icons it passes.

## Install

```bash
kui init
kui add liquid-tab-bar
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::liquid_tab_bar::{
    LiquidTabBar, LiquidTabChanged, LiquidTabIcon, LiquidTabItem, liquid_panel_id,
};

let control = cx.new(|cx| {
    LiquidTabBar::new(
        vec![
            LiquidTabItem::new("home", "Home", LiquidTabIcon::Home),
            LiquidTabItem::new("saved", "Saved", LiquidTabIcon::Bookmark),
            LiquidTabItem::new("inbox", "Inbox", LiquidTabIcon::Inbox)
                .with_badge(2)
                .with_badge_label("2 unread"),
        ],
        "home",
        cx,
    )
    .with_id("main-sections")
});
cx.subscribe(&control, |bar, event: &LiquidTabChanged, cx| {
    // Update the corresponding host panel as part of accepting the new section.
    bar.update(cx, |bar, cx| bar.set_value(event.0.clone(), cx));
}).detach();
let home_panel_id = liquid_panel_id("main-sections", "home");
```

Use `crate::ui` as the import root when installing source. The host mounts the panel for the current value. Use `liquid_panel_id` for its ID and accessibility identifier, and supply the panel's name and `TabPanel` role. `liquid_tab_id` identifies its tab. GPUI exposes each identifier independently.

| API | Default and behavior |
| --- | --- |
| `LiquidTabBar::new` | Requires ordered `LiquidTabItem` values and the initial selected value. Each tab value must be unique. |
| `with_id`, `with_label` | Uses a generated ID and the localized “Sections” label by default. |
| `with_labels`, `set_labels` | `Active` opens only the selected caption. `Always` shows every label. `None` keeps icon slots at 44px. |
| `with_active_icon` | Defaults to a filled built-in icon in the lens. Pass a separate icon for other artwork. `LiquidTabIcon::Svg` takes a monochrome SVG asset path. |
| `with_badge`, `with_badge_label` | Optional count, hidden at zero. The accessible name uses the supplied badge label or the localized count. |
| `set_tabs` | Updates options and rolls changed badge digits for retained tab values. |
| `LiquidTabChanged`, `set_value` | The control emits selection requests. The host accepts them by supplying the new value and updating its panel. |

Arrow Left and Right select and focus the adjacent tab with wraparound. Home and End select the first and last tab. Tap a tab or drag from the selected one. Releasing a horizontal drag chooses the landing slot. A quick flick moves at least one slot when there is room. Vertical touch movement remains available to the surrounding page.

The bar is 48px tall. It measures caption text and caps its viewport at the available width. Use `None` when several long captions compete for space. Hover lean, lens travel, and label opening settle immediately under reduced motion.
