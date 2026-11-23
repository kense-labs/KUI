# Files

Organize workspace folders and miniature document/spreadsheet templates across Grid and sortable List views with inline rename and Quick Look preview.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add files`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::files::{FilesScreen, FilesEvent};

let screen = cx.new(|cx| FilesScreen::new("files", cx));
cx.subscribe(&screen, |_, event: &FilesEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K or arrows move across folders and templates, Space toggles Quick Look inspector, R renames inline, V switches Grid/List, and / focuses search.

## API

Primary GPUI type: `FilesScreen` (`kui::components::files::FilesScreen`).
Event type: `FilesEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
