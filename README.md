# KUI

**KUI** is a Rust-native UI kit for [GPUI](https://gpui.rs). Applications own the source: install controls with the `kui` CLI into a local `ui/` tree.

This repository ships a **button MVP** — foundation tokens plus a finished `Button` — as the first registry item.

## Why KUI

- **GPUI native** — GPU UI, pinned `gpui-pre =0.3.10`
- **Copy the source** — no runtime component crate lock-in for your chrome
- **Behavior vs presentation** — primitives stay unstyled; components ship finished defaults
- **Preview = install source** — `apps/story` depends on the same `registry/` tree

## Quick start

```bash
# Check the KUI registry library (no Xcode Metal required)
cargo check -p kui

# Story smoke (headless)
cargo run -p kui-story

# Native window (needs GPUI platform / Xcode Metal on macOS)
cargo run -p kui-story --features native

# CLI
cargo install --path tools/cli
kui init
kui add button
kui doctor
```

## Install a button into your app

```bash
kui init
kui add button
```

```rust
use crate::ui::components::button::Button;

Button::new("save").primary().label("Save")
```

## Layout

```text
registry/           # authoritative KUI source (crate: kui)
apps/story/         # gallery — same source
tools/cli/          # kui init | add | doctor | sync-mod
tools/registry-build/
website/public/r/   # registry JSON
skills/kui/         # agent skills
docs/               # contracts + per-item markdown
```

## Brand

Use **KUI** in product copy and **`kui`** in crate names, CLI, modules, and paths. See [`website/public/assets/BRAND.md`](./website/public/assets/BRAND.md).

## Engineering

- [`AGENTS.md`](./AGENTS.md) — agent and contributor rules (Microsoft Pragmatic Rust Guidelines)
- [`DESIGN.md`](./DESIGN.md) — chrome tokens, Button metrics, interaction

## License

Apache-2.0.
