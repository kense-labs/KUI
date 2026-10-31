# KUI Design

This file is the only product design spec for KUI chrome. Studio-facing docs, story gallery, and installed components must match these roles. Do not add a second `DESIGN.md` under an app or package.

Token values are implemented in [`registry/src/foundation/tokens.rs`](./registry/src/foundation/tokens.rs) and consumed by Button chrome in [`registry/src/components/button/style.rs`](./registry/src/components/button/style.rs). After changing a table here, update those sources in the same change.

Brand naming (**KUI** / **`kui`**) lives in [`website/public/assets/BRAND.md`](./website/public/assets/BRAND.md). This document owns look and interaction, not org logos.

## 1. Direction

KUI chrome is a quiet work surface. Structure comes from type size, neutral fields, and clear pressed/disabled states. Color is reserved for meaning (accent action, danger), not decoration.

- Light mode is the authored default for MVP.
- Dark mode exists as a full token set and must stay contrast-safe when toggled in story.
- Components read semantic tokens (`accent`, `surface`, `danger`). They do not hard-code brand hex in call sites.
- Motion explains cause and effect. When `Theme.reduced_motion` is true, durations collapse to zero.
- Sentence case for labels. Regular and medium weights only. No eyebrow labels, no emoji in chrome.
- **Desktop and WASM share the same chrome.** Do not author a second web-only Button look. Platform notes live in [`docs/PLATFORMS.md`](./docs/PLATFORMS.md).

## 2. Color

Values below are the light theme unless noted. Dark counterparts live in `ColorTokens::dark()`.

| Role | Light | Dark | Use |
| --- | --- | --- | --- |
| Background | `#fafafa` | `#09090b` | Window / page fill |
| Surface | `#ffffff` | `#18181b` | Raised controls, cards |
| Surface muted | `#f4f4f5` | `#27272a` | Secondary button fill |
| Foreground | `#18181b` | `#fafafa` | Primary ink |
| Text secondary | `#52525b` | `#a1a1aa` | Supporting copy |
| Text muted | `#71717a` | `#71717a` | Placeholders, hints |
| Border | `#e4e4e7` | `#27272a` | Resting outlines |
| Border strong | `#a1a1aa` | `#52525b` | Emphasized outlines |
| Accent | `#18181b` | `#fafafa` | Primary button fill |
| Accent foreground | `#fafafa` | `#18181b` | Ink on accent |
| Accent subtle | `#e4e4e7` | `#27272a` | Soft accent wash |
| Danger | `#dc2626` | `#f87171` | Destructive actions |
| Danger foreground | `#ffffff` | `#18181b` | Ink on danger |
| Success | `#16a34a` | `#4ade80` | Positive status |
| Warning | `#d97706` | `#fbbf24` | Caution status |
| Focus outline | `#2563eb` | `#60a5fa` | Keyboard focus only |

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
| Button Lg | 16px | regular | Large button label |
| Button Md | 14px | regular | Default button label |
| Button Sm | 12px | regular | Compact button label |

Line height for button labels: 20px.

## 4. Shape and space

| Token | Value | Use |
| --- | --- | --- |
| Radius sm | 4px | Dense chips (future) |
| Radius md | 8px | **Button** corner radius |
| Radius lg | 12px | Cards / dialogs (future) |
| Radius full | 9999px | Pill controls (future) |
| Space 1–8 | 4 / 8 / 12 / 16 / 24 / 32px | Padding and gaps |

Concentric radii: nested surfaces use the next smaller radius step when both are rounded.

## 5. Button (MVP)

### Variants

| Variant | Fill | Ink | Border |
| --- | --- | --- | --- |
| Primary | `accent` | `accent_foreground` | none |
| Secondary | `surface_muted` | `foreground` | `border` 1px |
| Ghost | transparent | `foreground` | none |
| Danger | `danger` | `danger_foreground` | none |

### Sizes

| Size | Pad X | Pad Y | Label |
| --- | --- | --- | --- |
| Sm | 10px | 4px | 12px |
| Md (default) | 14px | 6px | 14px |
| Lg | 18px | 8px | 16px |

Corner radius is always `radius.md` (8px) for every size.

### States

| State | Behavior |
| --- | --- |
| Rest | Variant chrome as above |
| Hover / press | Prefer `MOTION.snappy` scale or opacity feedback when motion is enabled; no change when `reduced_motion` |
| Disabled | Opacity 50%; ignore clicks |
| Loading | Label becomes `…`; opacity 50%; ignore clicks |
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
- Do not add decorative gradients, glows, or multi-layer shadows on Button.
- Do not ship Tailwind / CSS-module chrome; KUI paints through GPUI on desktop and canvas/WebGPU on web.
- Do not copy Magic UI / marketing-effect patterns into application chrome.
- Do not maintain a separate DOM React tree for KUI controls.

## 10. Change process

1. Edit the tables in this file.
2. Update `registry/src/foundation/tokens.rs` and affected component `style.rs` files.
3. Update `docs/items/button.md` if public behavior or metrics change.
4. Run story smoke (`cargo test -p kui-story`) and visual check with `--features native` when available.
