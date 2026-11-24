# Client context

Manage per-client live financial KPIs, reports, files, meeting recaps, and team notes in one shared workspace block.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add client-context`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::client_context::{ClientContextBlock, ClientContextEvent};

let workspace = cx.new(|cx| {
    ClientContextBlock::new("client-context", cx)
        .with_me("Lena Hart")
});
cx.subscribe(&workspace, |_, event: &ClientContextEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Open `Switch client` to switch accounts or create a new client inline, click KPI cards to highlight a metric, switch between `Context`, `Reports`, `Financials`, and `Settings`, filter timeline entries by `All`, `Meetings`, `Emails`, `Files`, or `Notes`, type `@` in the composer to reference a client report, and press `Cmd+Enter` to post.

## API

Primary GPUI type: `ClientContextBlock` (`kui::components::client_context::ClientContextBlock`).
Event type: `ClientContextEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_clients(...)`, `with_active_client(...)`, `with_me(...)`, `with_filter(...)`, `with_tab(...)`, `with_loading(true)`, `with_empty(true)`, and responsive desktop-to-mobile adaptation.

## Motion

KPI numbers roll smoothly via `pro_rolling_number` when switching clients, segmented section tabs and timeline filter pills glide with `pro_pill_indicator`, report disclosures fold open via `pro_fold`, and popovers respect `Theme::reduced_motion`. Below `680px`, KPI cards wrap into a 2x2 grid and toolbars stack cleanly down to `390px`.

Desktop and web use the same GPUI source.
