# Morph nav

Expand a responsive navigation bar into section links or local search on one surface.

## Install

```bash
kui init
kui add morph-nav
```

Use `kui::` for the crate API and `crate::ui::` for copied source.

## Usage

```rust
use gpui::prelude::*;
use kui::components::morph_nav::{MorphNav, MorphNavItem, MorphNavLink, MorphNavSearchItem};

let docs = MorphNavLink::new("docs", "Documentation")
    .with_description("Guides and API reference")
    .with_href("/docs");
let search = MorphNavSearchItem::new(docs.clone()).with_section("resources");
let control = cx.new(|cx| {
    MorphNav::new("KUI", vec![
        MorphNavItem::new("resources", "Resources").with_links(vec![docs]),
        MorphNavItem::new("pricing", "Pricing").with_href("/pricing"),
    ], window, cx)
        .with_label("Main")
        .with_search("Search KUI", vec![search], window, cx)
});
```

Mount the entity after `kui::init(cx)`. Subscribe to `MorphNavNavigate` to route each destination. The event contains the destination ID, label, optional href and optional parent section. Supply the current section from the router with `set_current`. Drive `set_compact` from the host scroll state.

| Input | Behavior |
| --- | --- |
| `MorphNavItem` | Use stable IDs. A destination with links, a feature or a footer opens a section. A plain destination emits directly. |
| `with_links`, `with_columns` | Automatic uses two columns above three links when there is room. `One` and `Two` choose the preferred layout. Below a 420px viewport it becomes one column. |
| `with_feature`, `with_footer` | Add an actionable featured destination and a footer destination. Links accept optional descriptions, glyphs and hrefs. Use `with_feature_art` to add a decorative GPUI view. |
| `with_brand`, `with_action` | Set the leading brand and trailing primary action. An empty brand name omits the brand button. |
| `with_search` | Optional. Takes an independent index with optional group, parent section and keywords. Empty query shows the first five entries. Search returns at most six. |
| `with_collapse_below` | Default 720px of container width. Under it, Menu opens a grouped list of destinations. |
| `with_label`, `with_current`, `with_compact` | Default landmark name Main. The current section starts unset and compact mode starts off. |
| `set_visible` | Tell the nav when its host hides it so it stops requesting animation frames. |

Arrow Left and Right move among the top-level items. Home and End move to the edges. Arrow Down or Enter opens a section and focuses its first link. The up and down keys navigate the panel or search results. Escape closes the panel or search and restores the triggering focus.

Mouse hover waits 60ms before opening and 180ms before closing on exit. Touch taps open directly. The panel changes size from the current height and slides in the direction of section changes. With reduced motion, layout settles immediately.

Desktop and WASM use the same GPUI view. The surface uses KUI's 8px default chrome.
