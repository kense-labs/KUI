# Glass card

Show a featured person, product, or place in a 4:5 photo card with a glass summary. Tap the photo or header to reveal a short detail section over the same image. The pointer lights the panel rim as it moves.

## Installation

```sh
kui add glass-card
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::glass_card::{GlassCard, GlassCardOpenChanged};

let card = cx.new(|cx| {
    GlassCard::new("images/jasmine.jpg", "Portrait of Jasmine Brooks", "Jasmine Brooks", cx)
        .with_id("featured-profile")
        .with_subtitle(Box::new(|_, _| gpui::div().child("Design lead, Lisbon").into_any_element()))
        .with_aside(Box::new(|_, _| gpui::div().child("★ 4.9").into_any_element()))
        .with_body(Box::new(|_, _| {
            gpui::div().child("Design systems and motion.").into_any_element()
        }))
        .with_open(false)
});
window
    .subscribe(&card, cx, |card, event: &GlassCardOpenChanged, w, cx| {
        card.update(cx, |card, cx| card.set_open(event.0, w, cx));
    })
    .detach();
```

Register the referenced image path with the host asset source. A file path, URL, or loaded `ImageSource` works when supported by the host asset loader.

## API

| Input | Default | Behavior |
| --- | --- | --- |
| `new(image, image_alt, title, cx)` | Required | Image, accessible alternative (empty for decorative images), and header title. |
| `with_subtitle(Content)`, `with_aside(Content)` | None | Optional summary and trailing slots. |
| `with_body(Content)` | None | Optional detail content built only while expanded. Actions inside can handle their own events. |
| `with_width(f32)` | 360px | Preferred width clamped by the available layout. Height follows 4:5. |
| `with_style(Fn(Div) -> Div)` | None | GPUI refinement of the outer layout. |
| `with_tilt(f32)` | 6° | Controls the amount of pointer-driven artwork parallax. Zero removes parallax. |
| `with_solid_panel(bool)` | `false` | Use an opaque panel for hosts honoring reduced transparency. |
| `with_default_open(bool)` | `false` | Initial state for an uncontrolled card. |
| `with_open(bool)` | None | Controlled mode. Receive `GlassCardOpenChanged(bool)` and apply accepted state with `set_open`. |
| `with_id(ElementId)` | Title | Supply a stable, unique ID when multiple cards share a title. |

## Interaction and motion

- Tap the photo or the header. Enter and Space activate the focused header. Tab reaches detail actions in order while the card is open. Escape collapses it and returns focus to the header.
- The glass panel is inset 12px and uses the shared 8px surface radius. The header is 73px tall when a subtitle is supplied, with 16px horizontal padding. Detail height follows the supplied content. Tall details scroll inside the card.
- The height follows a 420ms spring that keeps the bottom edge anchored and accepts immediate reversal. The photo zooms from 1.001× to 1.05× while the floor shade deepens. Details fade in over 180ms and rise 6px. Reduced motion snaps geometry and keeps the fade.
- The pointer retargets two spring axes for photo parallax and the rim highlight. The panel keeps the same text layout and hitboxes across hover. Touch activates the disclosure without a hover tilt. The glass samples a blurred, saturated image tile cached by image ID. The card supports still image backgrounds.
