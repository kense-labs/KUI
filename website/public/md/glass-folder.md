# Glass folder

Render a glassy blue folder holding angled documents behind a frosted front pocket that tips open on hover.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add glass-folder`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::{prelude::*, px};
use kui::components::glass_folder::GlassFolder;

let folder = cx.new(|cx| {
    GlassFolder::new("kb-folder", cx)
        .with_width(px(360.))
        .with_document_count(3)
});
```

## Interaction

Hovering or focusing tips the frosted pocket forward while the inner document sheets fan upward and apart; clicking or pressing `Enter` / `Space` pins the open state and emits `GlassFolderEvent::OpenChanged`.

## API

Primary GPUI type: `GlassFolder` (`kui::components::glass_folder::GlassFolder`).
Event type: `GlassFolderEvent` (`OpenChanged(bool)`).

Supports light and dark `Theme` modes, custom accent colors (`with_color`), configurable document counts or caption labels (`with_document_count`, `with_documents`), bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, `with_empty(true)` empty states, and `3 : 2` responsive container scaling.

## Motion

Critically damped `Slide` spring (`0.55s`) drives the 3D-foreshortened pocket tilt, specular rim shift, and independent document sheet fan angles over a real-time separable-Gaussian frosted backdrop; `Theme::reduced_motion` snaps between closed and open poses immediately.

Desktop and web use the same GPUI source.
