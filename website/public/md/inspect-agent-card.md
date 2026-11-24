# Inspect agent card

Render an autonomous agent card tilted in 3D perspective with Figma-style `#0190fe` selection chrome, corner handles, dimension badges, and a red spacing guide walking row by row.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this item into your project, run `kui init` and `kui add inspect-agent-card`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::inspect_agent_card::InspectAgentCard;

let inspect = cx.new(|cx| {
    InspectAgentCard::new("hero-agent-inspect", cx)
        .with_hold(1600)
        .with_delay(600)
});
```

## Interaction

Automatically walks the pixel-inspection frame through `Header` (`16px`), `Task` (`12px`), and `Meta` (`8px`). Moving the pointer over any tilted row pins inspection to that row via exact inverse 3D homography; clicking pins/unpins, arrow keys step between rows, and `Enter` / `Space` pauses or resumes playback.

## API

Primary GPUI type: `InspectAgentCard` (`kui::components::inspect_agent_card::InspectAgentCard`).
Supporting types: `InspectAgent`, `InspectAgentStatus`, `InspectAgentRow`.
Event type: `InspectAgentCardEvent` (`PinRow(Option<InspectAgentRow>)`, `PlayingChanged(bool)`).

Supports light and dark `Theme` modes, `with_mirror(true)` left-tilted layout, `with_flat(true)` zero-tilt mode, bilingual `UiLanguage` (`English` / `Chinese`), `with_loading(true)` skeleton placeholders, and `with_empty(true)` empty states.

## Motion

`Slide` springs glide the `#0190fe` selection frame and diagonal padding hatch across rows while staggered reveal curves sequence the hand cursor, 4 corner resize handles, layer tag, `10` padding pills, `#ff3b30` spacing guide, and `W Fill 236 px` badge; `Theme::reduced_motion` holds inspection steady on `Header`.

Desktop and web use the same GPUI source.
