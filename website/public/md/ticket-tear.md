# Ticket tear

Redeem a ticket by pulling the paper stub along its perforation.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI before opening a window. To copy the component source, run `kui init` and `kui add ticket-tear`. Use `crate::ui::` for the copied imports.

## Usage

```rust
use gpui::{div, prelude::*, px};
use kui::components::ticket_tear::TicketTear;

let ticket = cx.new(|cx| {
    TicketTear::new(
        |_, _| div().p_5().h(px(211.)).child("Kense Day").into_any_element(),
        |_, _| div().size_full().flex().items_center().justify_center().child("Seat 12").into_any_element(),
        cx,
    )
    .with_stub_width(px(96.))
    .with_label("Tear off stub to check in")
});
```

## API

| Input or event | Type | Default |
| --- | --- | --- |
| Body and stub | Each `impl Fn(&mut Window, &mut App) -> AnyElement + 'static` | Required |
| `with_default_torn` | `bool` | `false` |
| `with_stub_width` | `Pixels` | 104px. The demo uses 96px plus a 5px seam overlap. |
| `with_label` | `impl Into<SharedString>` | Tear off stub, localized to the current KUI language |
| `with_paper` | `impl Into<Hsla>` | Current light or dark paper color |
| `with_style` | `impl Fn(Div) -> Div + 'static` | Fill available width. Content determines height. |
| `with_id` | `impl Into<ElementId>` | `ticket-tear` |
| `set_torn` | `(bool, &mut Context<Self>)` | `true` animates detachment. `false` replaces the stub and cancels in-flight motion. |
| `is_torn` | `() -> bool` | Becomes true at detachment. |
| `TicketTornChanged` | `TicketTornChanged(pub bool)` | User detachment emits `true` once. Host state changes are silent. |

Keep the ticket and stub layouts short enough for the viewport. Optionally install the `TicketTearPlatform` global to supply a gesture region and perforation haptics. The Web preview gates page scroll only for touches started on the active stub. Desktop and Web share the same geometry and state.

## Interaction

Pull right or down. The tear follows the pointer from the top. Short pulls leave the tear in place while the flap springs back. A long pull, a flick or Enter or Space on the focused stub finishes the tear. Escape or loss of window activation releases the current pull and preserves the tear. The sample's New ticket control becomes available after detachment.

## Motion

The paper uses 9px end notches and holes on a 10px pitch. Its torn outline crosses the holes and exposes fine fibers. The flap follows the pull on a spring. After detachment it drifts, rotates and falls while fading over 520ms. Reduced motion keeps the paper still and fades the stub in 160ms. The paper rotates while the caller's stub content tracks its center and opacity.
