# Workflow builder

Author branching AI automations on a zoomable dot-grid canvas with floating Nodes palette, Bezier YES/NO edges, live Run pulse, step inspector, and 5-tab Runs drawer.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add workflow-builder`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::workflow_builder::{WorkflowBuilderScreen, WorkflowBuilderEvent};

let screen = cx.new(|cx| WorkflowBuilderScreen::new("workflow-builder", cx));
cx.subscribe(&screen, |_, event: &WorkflowBuilderEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Press V/C for Select/Comment tools, A to toggle the Nodes palette, + / - / Shift+1 to zoom or fit, R or Cmd+Enter to execute a live test run, and Delete to remove.

## API

Primary GPUI type: `WorkflowBuilderScreen` (`kui::components::workflow_builder::WorkflowBuilderScreen`).
Event type: `WorkflowBuilderEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and responsive desktop-to-mobile adaptation.

## Motion

Interactive transitions, live status indicators, and skeleton sweeps follow KUI motion curves and respect Theme::reduced_motion. Responsive breakpoints adapt columns, toolbars, and side inspectors cleanly from full desktop down to 390px mobile viewports.

Desktop and web use the same GPUI source.
