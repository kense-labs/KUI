# Holo card

Display a membership, achievement, or collectible card with pointer lighting and a reverse face.

## Installation

```sh
kui add holo-card
```

## Usage

```rust
use gpui::prelude::*;
use kui::components::holo_card::{HoloCard, HoloCardFlipped, HoloFace};

let card = cx.new(|cx| {
    let front = HoloFace::new("Jasmine Brooks, member", |_, _| {
        gpui::div().size_full().p_4().child("Jasmine Brooks").into_any_element()
    });
    let back = HoloFace::new("Member since 2023", |_, _| {
        gpui::div().size_full().p_4().child("Member since 2023").into_any_element()
    });
    HoloCard::new(front, cx).with_back(back).with_flipped(false)
});
cx.subscribe(&card, |card, event: &HoloCardFlipped, cx| {
    card.update(cx, |card, cx| card.set_flipped(event.0, cx));
}).detach();
```

## API

| Method or event | Default | Behavior |
| --- | --- | --- |
| `HoloCard::new(HoloFace, &App)` | Required | Supplies the front face. |
| `HoloFace::new(label, Fn(&mut Window, &mut App) -> AnyElement)` | Required label | Places ordinary GPUI elements inside the projected face bounds. |
| `HoloFace::painted(label, Fn(HoloSurface, &mut Window, &mut App))` | Required label | Paints custom artwork and projected positions using normalized card coordinates. |
| `with_back(HoloFace)` | None | Enables flipping, keyboard focus and pressed state. |
| `with_flipped(bool)` / `with_default_flipped(bool)` | Uncontrolled front | Sets the controlled state or the initial uncontrolled state. |
| `set_flipped(bool, &mut Context<Self>)` / `is_flipped()` | Front | Applies or reads the current side. An acknowledgment emits no new request. |
| `HoloCardFlipped(bool)` | No initial event | Reports the new side or requests a host change in controlled mode. |
| `with_width(Pixels)` / `with_style(Fn(Div) -> Div)` | 300px / none | Refines the outer GPUI layout. The card retains its 5:7 ratio. |
| `with_max_tilt(f32)` | 14° | Limits tilt per axis. Accepts finite values within 0–45°. |
| `with_intensity(f32)` | 1 | Clamps the foil, sparkle and glare intensity to 0–1. |
| `with_gyroscope(bool)` / `HoloCardOrientationRequested` | Enabled / first tap | The host may request permission and start orientation input. |
| `set_device_tilt(Option<Point<f32>>, &mut Context<Self>)` | None | Applies normalized host tilt. Pass `None` when the sensor is unavailable. |
| `with_reduced_transparency(bool)` / `set_reduced_transparency(bool, &mut Context<Self>)` | False | Removes foil and sparkle while keeping glare. |
| `with_label(impl Into<SharedString>)` / `with_id(impl Into<ElementId>)` | Face label / front label | Overrides the accessibility name or GPUI identity. |

## Interaction and motion

- Move the pointer to tilt the card. The light follows on a softer spring and the shadow moves against the lean. Moving away restores the neutral pose.
- Tap, click, or press Enter or Space to flip a two-sided card. A flip rotates half a turn toward the activated side. An interrupted rotation keeps its instantaneous angle and velocity.
- The gallery calibrates device orientation from its first valid reading. With no sensor input, dragging a finger over the card tilts it. Movement outside the tap tolerance cancels the flip. Vertical browser scrolling remains available.
- Reduced motion removes tilt, rotation and glitter. Painted faces crossfade. Ordinary GPUI elements mount only on the current side, with an opacity transition, so hidden controls cannot receive keyboard focus.
- Painted artwork uses a 900px perspective plane. Ordinary GPUI elements are laid out within its projected bounding rectangle. Keep content within the face and supply descriptive text for artwork drawn to the canvas.
