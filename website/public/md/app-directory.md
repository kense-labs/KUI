# App directory

Browse, search, filter, and connect workspace integrations with category rails and an OAuth / API-key connect flow.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add app-directory`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::app_directory::{AppDirectoryBlock, AppDirectoryEvent};

let directory = cx.new(|cx| AppDirectoryBlock::new("app-directory", cx));
cx.subscribe(&directory, |_, event: &AppDirectoryEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press `/` to focus search, switch categories in the left rail (or chip strip on compact viewports), open `Filter` to narrow by connection or verification status, click `Connect` to authorize via `OAuth 2.0` or `API Key`, and press `Escape` to dismiss menus or dialogs.

## API

Primary GPUI type: `AppDirectoryBlock` (`kui::components::app_directory::AppDirectoryBlock`).
Event type: `AppDirectoryEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_apps(...)`, `with_categories(...)`, `with_title(...)`, `with_subtitle(...)`, `with_category(...)`, `with_status_filter(...)`, `with_loading(true)`, `with_empty(true)`, and responsive desktop-to-mobile adaptation.

## Motion

Category highlights and modal auth tabs glide via `pro_pill_indicator`, filtered grids transition with `pro_view_transition`, live connected counts roll via `pro_rolling_number`, and authorization dialogs settle smoothly while respecting `Theme::reduced_motion`. Responsive breakpoints switch from a 3-column grid with sidebar above `820px` to a 2-column chip layout and a 1-column mobile stack below `560px`.

Desktop and web use the same GPUI source.
