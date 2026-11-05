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
- Product site lockup is **Kense mark · KUI** only. Company site is **Kense mark · Kense**. Do not put both wordmarks in the product top bar.

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

The palette in the website header offers six presets and a custom color. Primary actions, keyboard focus, selection markers and subtle washes use the accent. Body text and status colors retain their semantic values.

| Palette | Light | Dark |
| --- | --- | --- |
| Neutral (default) | `#000000` | `#ffffff` |
| Blue | `#245bc4` | `#8bb4ff` |
| Violet | `#7142b6` | `#bc9cff` |
| Teal | `#08776d` | `#68d4c4` |
| Rose | `#b93c67` | `#ff94b3` |
| Amber | `#955a08` | `#f4be69` |

In the custom editor, drag the saturation/brightness pad, adjust the hue, or enter a three- or six-digit HEX color. The document and the mounted GPUI preview show the draft together. Apply stores the normalized `#rrggbb` under `kui-accent`; Cancel or Escape restores the saved selection. The last confirmed custom color remains available after choosing a preset. The custom fill uses the same HEX in both themes. Its foreground uses whichever of white or black has higher contrast.

Accent transitions take 180ms with cubic ease-out. A new selection starts from the displayed color. Reduced motion applies the new pair immediately. Rust hosts set a pair with `Theme::set_accent(AccentPalette { light, dark })`. Each tint's alpha controls how much of the underlying KUI surface it replaces.

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
| Radius md | 8px | Default corner for rectangular controls, menus, cards and docs surfaces |
| Radius sm / lg / panel | 8px | Compatibility names for the default surface radius |
| Radius full | 9999px | Capsule switches and period selectors; circular avatars, radio marks and status dots |
| Space 1–8 | 4 / 8 / 12 / 16 / 24 / 32px | Padding and gaps |

Control heights: sm **36px**, md **44px**, lg **50px**.

Exposed corners of nested rectangular surfaces use the same 8px radius. Joined edges stay square. A circle is drawn from its diameter.

### 胶囊开关例外 / Capsule controls

2026-11-05 用户要求：原本采用胶囊形的按钮开关恢复胶囊外观，不受统一 8px 圆角限制。不要再把所有控件机械地设为 `radius.md`。

- `Switch` 的 42×24px 轨道默认读取 `radius.full`，实际两端半径为 12px；18px 滑块保持圆形，按压加宽时保持胶囊。此规则同样适用于内联编辑的失败模拟、操作状态的网络模拟、设置页和 Cookie 设置中的开关。
- `BillingToggle` 的外轨道、每段命中/焦点轮廓、移动高亮及优惠标签均使用 `radius.full`。移动高亮的圆角、边框和阴影由当前实际尺寸推导，最大为短边的一半，变宽时仍保持圆润。
- `GlassSegmented` 的玻璃外轨道、每段命中/焦点轮廓及伸缩高亮使用 `radius.full`；36px / 44px 轨道和内部高亮保持胶囊两端，不受矩形 8px 默认值限制。
- 其他已有规格明确为胶囊形的切换控件保留其原始形态，并在对应组件文档中记录；新增控件也先区分矩形、胶囊或圆形。`radius.md` 不是全局强制覆盖。
- 普通矩形 Button、输入框、卡片、菜单，以及原本为矩形的 SegmentedControl / ToggleGroup 继续使用各自的 8px 默认外框；本例外不将它们批量改成胶囊。

Capsules use the shared full-radius token and clamp painted corners to half the current shorter dimension. Their fills, borders, shadows and focus feedback follow the same geometry; interaction, dimensions and motion retain the component contract.

## 5. Button (MVP)

All rectangular button surfaces and their highlights use the shared **8px** control radius. The preview canvas, install panel and code panel use the same radius. Primary fills follow the active accent.

Progress and result overlays must preserve their owning button's outline. In GPUI, rectangular overflow clipping does not round child fills: paint a full-size rounded face beneath the progress clip. A completed fill must not cover an 8px button with a square-cornered rectangle.

### Documentation preview and source

- Before the first GPUI frame, center a 20px loading ring and a muted 13px status label inside the preview. Use a 2px ring stroke, a 12px gap, and an 800ms rotation. Keep the preview's existing dimensions while loading.
- Respect reduced motion by keeping the loading ring static. Expose the loading message as a polite status and mark the preview body busy until it paints or reports an error.
- Hide loading only after the first frame paints. On a module or runtime loading failure, stop the ring and show a localized retry instruction.
- Usage examples, the Code tab, and every installed source file share the same escaped syntax markup and light/dark syntax colors. Preserve plain source in clipboard actions.
- Shell blocks distinguish commands, flags, variables, paths, quoted values, and comments with the same syntax palette.

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

Corner radius is always `radius.md` (**8px**) for every size.

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
- Do not add decorative gradients or glows on Button. A quiet resting shadow on hover is allowed; do not stack floating shadows.
- Do not ship Tailwind / CSS-module chrome; KUI paints through GPUI on desktop and canvas/WebGPU on web.
- Do not copy Magic UI / marketing-effect patterns into application chrome.
- Do not maintain a separate DOM React tree for KUI controls.

## 10. Change process

1. Edit the tables in this file.
2. Update `registry/src/foundation/tokens.rs` and affected component `style.rs` files.
3. Update `docs/items/button.md` if public behavior or metrics change.
4. Run story smoke (`cargo test -p kui-story`) and visual check with `--features native` when available.
