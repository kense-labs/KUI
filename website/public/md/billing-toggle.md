# Billing toggle

Choose a monthly or yearly billing period.

## Install

```bash
cargo add kense-kui --rename kui --no-default-features
```

Initialize KUI with `kui::init(cx)` before opening a window. To copy this component into your project, run `kui init` and `kui add billing-toggle`; replace `kui::` imports below with `crate::ui::`.

## Usage

```rust
use gpui::prelude::*;
use kui::components::billing_toggle::BillingToggle;

let control = cx.new(|cx| BillingToggle::new("Billing period", cx).with_savings("Save 20%"));
```

## Interaction

Use arrow keys to choose a period. Selecting the current period emits no event.

## API

`with_savings` · `(impl Into<SharedString>) -> Self`

Configure labels, the yearly savings note and the initial period. The host owns pricing and checkout.

Event: `BillingChanged`.

## Motion

The selected surface slides between periods; reduced motion jumps to the selection.

Desktop and web use the same GPUI view.

## Shape

The track, options, moving selection and savings badge use `Theme.tokens.radius.full`. Painted selection corners are clamped to half the current shorter dimension; its shadow follows the same capsule, including during width changes. This restores the original capsule shape and is an explicit exception to KUI's 8px rectangular surface rule (user instruction, 2026-11-05). Selection logic, measured bounds and spring parameters are unchanged.

## Verification

2026-11-05 web measurement (900×760, light and dark, `?story=billing-toggle`):

- Initial state selects Monthly; the surface is the raised segment and the unselected Yearly segment shows the green `Save 20%` badge. Clicking Yearly moves the surface to the wider right segment.
- Both position and width animate: left edge 274→351 and width 85→134 device px over 24 flattened frames (0.1× clock), monotonic and decelerating toward x≈357 / width≈137 — the two `duration(…, 0.34)` critical springs at `billing_toggle.rs:57-58` (ω = 15.4). The wider segment is therefore a smooth grow, not a snap.
- Reduced motion applies the whole change in one frame (274..358 / w85 → 357..493 / w137, no intermediate), matching the `reset` path.
- Keyboard (`→` after focusing Monthly) and touch (`Input.dispatchTouchEvent` on Yearly) drive the same move; re-selecting the current period leaves the surface in place and emits no event.
- Hover only dims the unselected label to `secondary` (`:300`); there is no separate pressed state because selection fires on release.
- 320px Chinese renders 计费周期 / 按月 / 按年 / 节省 20% unwrapped with no missing glyphs. At 125% zoom the Monthly surface is 80.8 CSS px wide (81 at 100%) and 31.2 tall (31), x unchanged — within 1 CSS px.
- Not compared frame-by-frame against a same-speed 60Hz baseline, so no frame-level parity is claimed; contrast values were not measured numerically.
