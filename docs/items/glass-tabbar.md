# Glass tab bar

Float a capsule tab list and a separate action above a scrollable surface. The selection lens moves with the pointer during a drag and settles on the nearest main tab on release.

## Installation

```sh
kui add glass-tabbar
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::glass_tabbar::{
    GlassTabBar, GlassTabBarCompactChanged, GlassTabBarItem, GlassTabBarValueChanged, GlassTabIcon,
};

let scroll = gpui::ScrollHandle::new();
let bar = cx.new(|cx| {
    GlassTabBar::new(
        vec![
            GlassTabBarItem::new("home", "Home", GlassTabIcon::Home),
            GlassTabBarItem::new("explore", "Explore", GlassTabIcon::Explore),
            GlassTabBarItem::new("saved", "Saved", GlassTabIcon::Saved).with_badge(3),
            GlassTabBarItem::new("profile", "Profile", GlassTabIcon::Profile),
        ],
        cx,
    )
    .with_label("Main")
    .with_action(GlassTabBarItem::new("search", "Search", GlassTabIcon::Search), cx)
    .with_scroll_handle(scroll.clone())
    .with_value("home")
    .with_compact(false)
});
cx.subscribe(&bar, |bar, event: &GlassTabBarValueChanged, cx| {
    bar.update(cx, |bar, cx| bar.set_value(event.0.clone(), cx));
}).detach();
cx.subscribe(&bar, |bar, event: &GlassTabBarCompactChanged, cx| {
    bar.update(cx, |bar, cx| bar.set_compact(event.0, cx));
}).detach();
```

Attach the same `ScrollHandle` to a GPUI scrollable viewport and position the bar above it. A host can instead forward scroll distances to `observe_scroll_offset`. Use the selection event to update the content panel.

## API

| Input | Default | Behavior |
| --- | --- | --- |
| `new(items, cx)` | Required | Items have unique values, accessible labels, and icons. Numeric or textual badges hide at zero or empty. |
| `with_action(item, cx)` | None | Separate icon button, in the same tab order. |
| `with_value(value)` | None | Controlled selection. Use `set_value` to accept requests. An unknown value or the action hides the main lens. |
| `with_default_value(value)` | First main item | Initial uncontrolled selection. |
| `with_compact(bool)` | None | Controlled size. Use `set_compact` to accept requests. |
| `with_default_compact(bool)` | `false` | Initial uncontrolled size. |
| `with_scroll_handle(handle)` | None | Track the host scroll region and let vertical touch pan through the bar. |
| `with_collapse_on_scroll(bool)` | `true` | Shrink after scrolling down; grow after scrolling up. |
| `with_label(label)` | `"Tabs"` | Name of the navigation and tab list. |
| `with_style(Fn(Div) -> Div)` | None | Refine positioning or layout of the outer GPUI element. |
| `with_backdrop(Fn() -> Option<GlassTabBackdrop>)` | None | Supply a texture, an optional blurred texture, and matching window bounds for the moving glass sample. |
| `with_solid_surface(bool)`, `with_increased_contrast(bool)` | `false` | Apply host accessibility preferences. |

## Interaction and motion

- Tab enters at the selected item. Left and Right move and select, wrapping through the action. Home and End select the first or last destination. Enter or Space selects and grows a compact bar.
- Press the selected main tab to lift the lens. Drag horizontally at least 4px to move it. Release to choose the closest main tab. Escape or a canceled touch returns to the prior selection. Vertical touch in a bound scroll region pans the content.
- Main tabs are up to 78×62px expanded and 48×48px compact. The action stays circular. Narrow layouts reduce slot width while keeping names and badges accessible.
- One 400ms spring drives slot widths, height, icon position and label fade. Lens speed drives lift and magnification. Reduced motion jumps to the target geometry and skips lift, stretch and magnification.
