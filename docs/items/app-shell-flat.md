# App shell flat

Lay out the workspace sidebar, continuous top header, main canvas, and work area edge-to-edge with hairline dividers.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add app-shell-flat`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::app_shell_flat::{AppShellFlat, AppShellFlatEvent};

let screen = cx.new(|cx| AppShellFlat::new("app-shell-flat", cx));
cx.subscribe(&screen, |_, event: &AppShellFlatEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Shares full keyboard command search (Cmd+K), sidebar collapse, drag-resizable panes, and multi-tab artifact switching with AppShell.

## API

Primary GPUI type: `AppShellFlat` (`kui::components::app_shell_flat::AppShellFlat`).
Event type: `AppShellFlatEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
