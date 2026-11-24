# Integration picker

Two-pane integrations page of a command menu — live search, a grouped list of SaaS services with connected dots, and a detail inspector with `Workspace` and `Personal` OAuth connection tiles.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add integration-picker`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::integration_picker::{IntegrationPicker, IntegrationPickerEvent};

let picker = cx.new(|cx| {
    IntegrationPicker::new("cmd-integrations", cx)
        .with_app_name("Kense")
        .with_selected("linear")
});
cx.subscribe(&picker, |_, event: &IntegrationPickerEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Use `↑` / `↓` to move across `Connected` and `Available` services, `Enter` to toggle the selected service, `⌘/` to focus the search input, `←` in an empty search box to go back, or toggle `Workspace` and `Personal` OAuth cards independently.

## API

Primary GPUI type: `IntegrationPicker` (`kui::components::integration_picker::IntegrationPicker`).
Event type: `IntegrationPickerEvent`.

Customize services with `with_services(Vec<PickerService>)`, product branding with `with_app_name`, initial selection with `with_selected`, and navigation chrome with `with_back_button`. Supports light and dark `Theme` modes, bilingual `UiLanguage`, responsive stacking below `560px`, `with_loading(true)`, and `with_empty(true)`.

## Motion

A spring-driven selection pill glides across `Connected` and `Available` sections, detail swaps lift smoothly on selection changes, and connected services radiate a calm outward ring around the Kense + Brand tiles. `Theme::reduced_motion` freezes ambient rings and snaps transitions immediately.

Desktop and web use the same GPUI source.
