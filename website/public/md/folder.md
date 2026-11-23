# Folder

Work inside a project folder with miniature file preview cards, workspace sharing & Notion/Drive connectors, custom project instructions, and scoped AI chat.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add folder`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::folder::{FolderScreen, FolderEvent};

let screen = cx.new(|cx| FolderScreen::new("folder", cx));
cx.subscribe(&screen, |_, event: &FolderEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

J/K or arrows select files, Enter/Space opens File Preview, C opens the scoped Folder Assistant, R renames, U uploads, and Backspace returns to Files.

## API

Primary GPUI type: `FolderScreen` (`kui::components::folder::FolderScreen`).
Event type: `FolderEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
