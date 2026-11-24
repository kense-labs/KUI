# Gradient text

Cut headline words from an ever-flowing four-blob liquid gradient with an optional left-to-right entrance sweep.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add gradient-text`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::{FontWeight, prelude::*, px};
use kui::components::gradient_text::GradientText;

let payoff = GradientText::new("hero-payoff", "feel.")
    .with_font_size(px(56.))
    .with_font_weight(FontWeight::SEMIBOLD)
    .with_speed(8.0)
    .with_sweep(true);
```

## Interaction

Preserves real accessible headline text beneath the painted glyph slices, automatically switches default pink-to-violet stops between light and dark `Theme` modes, and freezes on the finished gradient when reduced motion is enabled.

## API

Primary GPUI types: `GradientText`, `GradientTextShowcase` (`kui::components::gradient_text`).
Event type: `None`.

Supports light and dark `Theme` modes, custom 4-stop `Hsla` or hex palettes (`with_colors` / `with_hex_colors`), configurable cycle speed (`with_speed`), replayable `1100ms` sweep (`with_sweep` / `replay`), and bilingual `UiLanguage` (`English` / `Chinese`).

## Motion

Three radial color blobs orbit smoothly across a `100deg` two-stop base loop while GPUI's native `ShapedLine` glyphs are painted through clipped vertical slices for zero font or baseline mismatch with surrounding copy. On first view (and on replay), color wipes from left to right over `1100ms` along `cubic-bezier(0.65, 0.0, 0.25, 1.0)`; `Theme::reduced_motion` displays the full gradient standing still.

Desktop and web use the same GPUI source.
