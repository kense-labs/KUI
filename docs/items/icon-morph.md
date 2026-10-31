# Icon morph

Eight reversible icon pairs, including playback, alerts, details and likes.

`kui add icon-morph`

## Usage

```rust
use gpui::prelude::*;
use crate::ui::components::icon_morph::IconMorph;
use crate::ui::primitives::glyph::Glyph;

let control = cx.new(|_| IconMorph::new("player", [Glyph::Play, Glyph::Pause], ["Play", "Pause"]).with_captions(["Paused", "Playing"]));
```

Mount the entity as a GPUI child. Enter, Space or a tap toggles the pair. Fast reversals continue from the current stroke positions.

Each pair has its own stroke or fill motion. Rapid reversals continue from the current position; narrow screens use two columns.

## Interface

The control uses the theme’s `radius.md` (8px by default).

- Event: `MorphChanged(bool)`.
- API: `set(on, cx) / with_captions([S; 2])`.
- Implementation: `registry/src/components/icon_morph.rs`.
- Preview: [English](/en/kui/components/icon-morph/) · [中文](/kui/components/icon-morph/).
