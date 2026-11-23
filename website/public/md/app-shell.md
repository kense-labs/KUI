# App shell

Frame multi-column AI and product workspaces with a collapsible sidebar, command search, and tabbed work area.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add app-shell`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::app_shell::{AppShell, AppShellEvent, AppShellPreset};

let shell = cx.new(|cx| {
    AppShell::new("workspace-shell", cx)
        .with_preset(AppShellPreset::Agent, cx)
        .with_active("Activity")
        .with_work_area(true, true)
});
cx.subscribe(&shell, |_, ev: &AppShellEvent, _| {
    let _ = ev;
}).detach();
```

## Interaction

Press Cmd+K for command search, drag column splitters to resize, double-click a splitter to reset width, and fold either rail from its header.

## API

Primary GPUI type: `AppShell` (`kui::components::app_shell::AppShell`).
Event type: `AppShellEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Sidebar and work-area drawers adapt cleanly below 860px and 680px; reduced motion disables过渡 sweeps and snaps layout changes.

Desktop and web use the same GPUI source.
