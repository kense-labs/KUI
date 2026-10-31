# KUI Design

This file is the only product design spec for KUI chrome. Studio-facing docs, story gallery, and installed components must match these roles. Do not add a second `DESIGN.md` under an app or package.

Token values are implemented in [`registry/src/foundation/tokens.rs`](./registry/src/foundation/tokens.rs) and consumed by Button chrome in [`registry/src/components/button/style.rs`](./registry/src/components/button/style.rs). After changing a table here, update those sources in the same change.

Brand naming: **Kense** = company; **KUI** / **`kui`** = this product (see [`website/public/assets/BRAND.md`](./website/public/assets/BRAND.md)). This document owns look and interaction, not the product catalog.

## 1. Direction

KUI chrome is **pure black and white** by default: white page, black ink, black primary controls. No gradients, glows, or multi-layer shadows. Site and GPUI chrome share the same roles.

- Light mode is the authored default for MVP.
- Dark mode is inverted pure black/white and must stay contrast-safe when toggled in story.
- Components read semantic tokens (`accent`, `surface`, `danger`). They do not hard-code hex in call sites.
- **Kense brand `#171717` is not the everyday accent.** Use it only for important brand moments (logo lockup, org attribution, rare brand CTAs). See [`website/public/assets/BRAND.md`](./website/public/assets/BRAND.md).
- Motion explains cause and effect. When `Theme.reduced_motion` is true, durations collapse to zero.
- Sentence case for labels. Regular and medium weights only. No eyebrow labels, no emoji in chrome.
- **Desktop and WASM share the same chrome.** Do not author a second web-only Button look. Platform notes live in [`docs/PLATFORMS.md`](./docs/PLATFORMS.md).
- Brand lockups (site / story chrome) use a **flat parallel** row: Kense mark · KUI · optional org label.

## 2. Color

Values below are the light theme unless noted. Dark counterparts live in `ColorTokens::dark()`.

| Role | Light | Dark | Use |
| --- | --- | --- | --- |
| Background | `#ffffff` | `#000000` | Window / page fill |
| Surface | `#ffffff` | `#000000` | Raised controls, cards |
| Surface muted | `#f5f5f5` | `#1a1a1a` | Soft washes |
| Foreground | `#000000` | `#ffffff` | Primary ink |
| Text secondary | `#525252` | `#a3a3a3` | Supporting copy |
| Text muted | `#737373` | `#737373` | Placeholders, hints |
| Border | `#e5e5e5` | `#2a2a2a` | Resting outlines |
| Border strong | `#a3a3a3` | `#525252` | Emphasized outlines |
| Accent | `#000000` | `#ffffff` | Primary button fill |
| Accent foreground | `#ffffff` | `#000000` | Ink on accent |
| Accent subtle | `#f5f5f5` | `#1a1a1a` | Soft accent wash |
| Danger | `#b42318` | `#f97066` | Destructive actions |
| Danger foreground | `#ffffff` | `#000000` | Ink on danger |
| Success | `#16a34a` | `#4ade80` | Positive status |
| Warning | `#d97706` | `#fbbf24` | Caution status |
| Focus outline | `#000000` | `#ffffff` | Keyboard focus only |

Brand-only (not a default token in `ColorTokens`): Kense `#171717` / on-brand `#ffffff` — logo lockup and brand moments only.

### Focus policy

- Pointer focus draws no decorative ring.
- Keyboard focus uses the shared focus outline token (`focus_outline`) via `:focus-visible`-equivalent GPUI focus styling when wired.
- Disabled controls use 50% opacity and do not fire activation handlers.

## 3. Type

GPUI system UI font for MVP. Do not introduce a second display face for chrome.

| Role | Size | Weight | Use |
| --- | --- | --- | --- |
| Story title | 22px | regular | Gallery heading (“KUI”) |
| Subtitle | 14px | regular | Supporting line under title |
| Button label | 14px | medium (500) | All button sizes (Arc `text-sm`) |

Line height for button labels: 20px (`leading-body` ≈ 1.4).

## 4. Shape and space

| Token | Value | Use |
| --- | --- | --- |
| Radius sm | 4px | Dense chips (future) |
| Radius md | 4px | **Button** / control radius (tight, not soft) |
| Radius lg | 8px | Cards / dialogs (future) |
| Radius full | 9999px | Pill controls (future) |
| Space 1–8 | 4 / 8 / 12 / 16 / 24 / 32px | Padding and gaps |

Control heights (Arc): sm **36px**, md **44px**, lg **50px**.

Concentric radii: nested surfaces use the next smaller radius step when both are rounded.

## 5. Button (MVP)

Visual rhythm matches ArcUI Button (taller control, medium weight, outlined danger) with a tight **4px** radius (not soft, not pill). Colors stay KUI pure black/white; brand `#171717` is not used on everyday buttons.

### Variants

| Variant | Fill | Ink | Border |
| --- | --- | --- | --- |
| Primary | `accent` | `accent_foreground` | `accent` 1px |
| Secondary | `surface` | `foreground` | `border` 1px |
| Ghost | transparent | `text_secondary` | transparent |
| Danger | `surface` | `danger` | `border` 1px (outlined) |

### Sizes

| Size | Min height | Pad X | Label |
| --- | --- | --- | --- |
| Sm | 36px | 12px | 14px medium |
| Md (default) | 44px | 16px | 14px medium |
| Lg | 50px | 20px | 14px medium |

Corner radius is always `radius.md` (**4px**) for every size.

### States

| State | Behavior |
| --- | --- |
| Rest | Variant chrome as above |
| Hover / press | Prefer `MOTION.snappy` scale (~0.97) or opacity feedback when motion is enabled; no change when `reduced_motion` |
| Disabled (primary) | Opacity **52%**; ignore clicks |
| Disabled (secondary / danger / ghost) | Keep shape; mute label / fill (`surface_muted` / `text_muted`); ignore clicks |
| Loading | Label becomes `…`; treat as inert like disabled |
| Focus (keyboard) | Focus outline token; never a pointer-only ring |

### Interaction

- Activate with pointer click, Enter, and Space when focused.
- Accessible name is the visible label unless a host sets a different name.
- Keep a stable `ElementId` for the lifetime of the control instance.

### API shape (product)

```rust
Button::new("save")
    .primary()
    .label("Save")
    .into_themed(cx)
```

Builders: `primary` / `secondary` / `ghost` / `danger`, `with_size`, `disabled`, `loading`, `on_click`, `into_themed`.

## 6. Motion

Presets in `foundation::MOTION`:

| Preset | Duration | Use |
| --- | --- | --- |
| `instant` | 80ms | Micro fades |
| `snappy` | 120ms | Button press |
| `smooth` | 220ms | Panels (future) |
| `morph` | 280ms | Shared-element moves (future) |

When `Theme.reduced_motion` is true, effective duration is `0`.

## 7. Layout density (story)

- Gallery page padding: 24px (`space.six`).
- Control row gap: 8px (`space.two`).
- Vertical stack gap between sections: 16px (`space.four`).

## 8. Copy

- Sentence case: `Save changes`, not `Save Changes` or `SAVE CHANGES`.
- Buttons use verbs: `Save`, `Delete`, `Continue`.
- No trailing ellipsis except the loading glyph `…`.
- English default in source; localized hosts own translation.

## 9. Non-goals (MVP)

- Do not invent a second primary accent hue for navigation.
- Do not use Kense brand `#171717` as the default accent fill; reserve it for brand moments.
- License is MIT.
- Do not add decorative gradients or glows on Button. A quiet resting shadow on hover (Arc-style) is allowed later; do not stack floating shadows.
- Do not ship Tailwind / CSS-module chrome; KUI paints through GPUI on desktop and canvas/WebGPU on web.
- Do not copy Magic UI / marketing-effect patterns into application chrome.
- Do not maintain a separate DOM React tree for KUI controls.

## 10. Change process

1. Edit the tables in this file.
2. Update `registry/src/foundation/tokens.rs` and affected component `style.rs` files.
3. Update `docs/items/button.md` if public behavior or metrics change.
4. Run story smoke (`cargo test -p kui-story`) and visual check with `--features native` when available.
