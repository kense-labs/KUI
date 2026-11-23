# CRM

Manage accounts, deals, contacts, and renewals across inline-editable data tables and pipeline boards with undo/redo and an account activity inspector.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add crm`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::crm::{CrmScreen, CrmEvent};

let screen = cx.new(|cx| CrmScreen::new("crm", cx));
cx.subscribe(&screen, |_, event: &CrmEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press / to search, double-click a table cell to edit inline, Cmd+Z / Cmd+Shift+Z to undo or redo, switch Table and Board modes, and press I to inspect.

## API

Primary GPUI type: `CrmScreen` (`kui::components::crm::CrmScreen`).
Event type: `CrmEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
