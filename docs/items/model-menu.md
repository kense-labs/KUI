# Model menu

Render a frontier model picker product scene with provider marks, a gliding rose highlight band, and soft right-edge and foot fades.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add model-menu`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::model_menu::ModelMenu;

let menu = cx.new(|cx| {
    ModelMenu::new("hero-models", cx)
        .with_selected(1)
        .with_hold(2200)
        .with_walk(3)
});
```

## Interaction

Auto-walks the warm rose highlight band across the top `walk` models by default; hovering a row glides the band to that model, clicking or pressing `Enter` picks it (`ModelMenuEvent::Select`), `Up` / `Down` step through rows, and `Space` pauses or resumes auto-play.

## API

Primary GPUI type: `ModelMenu` (`kui::components::model_menu::ModelMenu`).
Supporting item type: `MenuModel`.
Event type: `ModelMenuEvent` (`Select { index, name }`).

Supports light and dark `Theme` modes, bilingual `UiLanguage` (`English` / `Chinese`), custom model rows and 16px marks (`with_models`, `MenuModel::with_mark`), optional header (`with_title`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

A `0.52s` `Slide` spring glides both the horizontal rose gradient band and its checkmark from row to row while true alpha edge masks dissolve the card's right border and lower rows into any host background; `Theme::reduced_motion` holds the band steady on the selected row.

Desktop and web use the same GPUI source.
