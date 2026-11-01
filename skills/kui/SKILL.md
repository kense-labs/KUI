---
name: kui
description: Build GPUI desktop and WASM UI with KUI components from crates.io or copied source. Use when installing or customizing KUI controls, especially Button.
---

# KUI

KUI is a Rust UI kit on GPUI. Use the `kense-kui` component library or copy selected registry items instead of inventing controls.

Read repository root [`AGENTS.md`](../../AGENTS.md) and [`DESIGN.md`](../../DESIGN.md) before changing Rust or chrome. Rust follows Microsoft [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/).

## Always

- Brand: product **KUI**, crates.io package **`kense-kui`**, Rust library/CLI/module **`kui`**
- Pin GPUI to `gpui-pre =0.3.10`
- Install the library with `cargo add kense-kui --rename kui --no-default-features`; use `kui::` imports
- Call `kui::init(cx)` before windows; never enable `test-support` on wasm32
- To own source, install the CLI with `cargo install --git https://github.com/kense-labs/KUI --locked kui-cli`, then run `kui init` and `kui add <id>`; use `crate::ui::` imports
- Use semantic theme tokens from `DESIGN.md`, not hard-coded brand colors in new code
- Honor `Theme.reduced_motion`
- Keep controls host-agnostic: native desktop and WASM web share `registry/` (see `docs/PLATFORMS.md`)
- Public modules and items use rustdoc summaries and canonical sections (M-MODULE-DOCS, M-CANONICAL-DOCS)

## Catalog

`tools/catalog` defines 108 installable components plus `kui-foundation`.
The library exposes the same controls under `kui::components`.

## Button

```rust
use kui::components::button::Button;

Button::new("save").primary().label("Save").into_themed(cx)
```

For CLI-installed source, use `crate::ui::components::button::Button` instead.

Read `docs/items/button.md` before changing behavior.
