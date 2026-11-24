# Agent gallery

Showcase multiple autonomous agents as slim vertical gradient panels flanking one live open product workspace screen.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add agent-gallery`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::agent_gallery::{AgentGalleryBlock, AgentGalleryEvent};

let block = cx.new(|cx| {
    AgentGalleryBlock::new("agent-gallery", cx)
        .with_title("Agents that run your playbook")
});
cx.subscribe(&block, |_, event: &AgentGalleryEvent, _| {
    let _ = event;
}).detach();
```

## Interaction

Click any slim gradient side panel or use the Left/Right arrow keys and footer chevron buttons to glide that agent open into the center showcase. Inside the open workspace screen, switch between `Runs`, `Tools`, `Context`, and `Settings` or add a new context note inline.

## API

Primary GPUI type: `AgentGalleryBlock` (`kui::components::agent_gallery::AgentGalleryBlock`).
Event type: `AgentGalleryEvent`.

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), `with_title(...)`, `with_subtitle(...)`, `with_items(...)`, `with_default_index(...)`, `with_loading(true)`, `with_empty(true)`, and responsive desktop-to-mobile adaptation.

## Motion

Picked panels widen smoothly into the center showcase via critically-damped `Slide` springs while the previously open panel folds back into a slim strip; side-panel artwork blooms to full saturation on hover and respects `Theme::reduced_motion`. Responsive breakpoints adapt panel widths and flanking gutters from `894px` desktop stages down to `390px` mobile viewports.

Desktop and web use the same GPUI source.
