# Mega menu morph

Navigate grouped destinations from a shared animated menu.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add mega-menu-morph`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::mega_menu_morph::{MegaMenuGroup, MegaMenuLink, MegaMenuMorph, MegaMenuSection};

let sections = vec![MegaMenuSection::new("products", "Products").with_groups(vec![MegaMenuGroup::new("Build", vec![MegaMenuLink::new("components", "Components")])])];
let control = cx.new(|cx| MegaMenuMorph::new("Kense", sections, cx).with_primary("start", "Get started"));
```

## Interaction

Hover or activate a section. Use arrow keys to navigate, Enter to choose and Escape to close. Groups and the feature card sit in columns when there is room. Narrow panels stack their content, and the compact header opens a scrollable accordion sheet.

## API

`with_primary` · `(impl Into<SharedString>, impl Into<SharedString>) -> Self`

Provide sections, groups and stable link IDs; optionally configure a brand destination, primary action and feature card. MegaMenuNavigate reports the chosen ID and the host performs navigation.

Event: `MegaMenuNavigate`.

## Motion

One panel moves and changes size on springs while menu contents crossfade; reduced motion suppresses movement.

Desktop and web use the same GPUI view.
