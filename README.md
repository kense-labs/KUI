# KUI

**KUI** is a **Kense** product — a Rust UI kit for [GPUI](https://gpui.rs) that runs on **native desktop** and **WASM web** from one control source. Applications own the source: install controls with the `kui` CLI into a local `ui/` tree.

This repository ships foundation tokens and 32 installable components, including buttons, gestures, menus, text fields, selectors, overlays and workspace layout. Browse the component families on the bilingual component site.

## Why KUI

- **Dual runtime** — macOS / Windows / Linux desktop and `wasm32-unknown-unknown` web
- **GPUI** — GPU UI, pinned `gpui-pre =0.3.10`
- **Copy the source** — no runtime component crate lock-in for your chrome
- **Behavior vs presentation** — primitives stay unstyled; components ship finished defaults
- **Preview = install source** — story hosts depend on the same `registry/` tree

## Quick start

```bash
# Check the KUI registry library (host CI; no Xcode Metal required)
cargo check -p kui

# Same library for WASM
cargo check -p kui --target wasm32-unknown-unknown --no-default-features

# Story smoke (headless)
cargo run -p kui-story

# Native desktop window (needs GPUI platform / Xcode Metal on macOS)
cargo run -p kui-story --features native

# WASM web gallery
./scripts/build-web.sh
python3 -m http.server -d apps/web/www 8080

# CLI
cargo install --path tools/cli
kui init
kui add button
kui doctor
```

Platform details: [`docs/PLATFORMS.md`](./docs/PLATFORMS.md).

## Local component website
From `website/`, run `bun install --frozen-lockfile` and `bun run dev`. Open `http://127.0.0.1:4321/kui/components/confirm-morph/` for the interactive story. Building or starting the website also builds its shared WASM preview bundle.

## Install a button into your app

```bash
kui init
kui add button
```

```rust
use crate::ui::components::button::Button;

Button::new("save").primary().label("Save")
```

The installed `ui/` source is host-agnostic. Your app Cargo.toml selects desktop `gpui_platform` or a WASM cdylib host.

## Layout

```text
registry/           # authoritative KUI source (crate: kui)
apps/story/         # shared gallery + desktop/smoke host
apps/web/     # WASM web host + www/
tools/cli/          # kui init | add | doctor | sync-mod
tools/registry-build/
website/public/r/   # registry JSON
skills/kui/         # agent skills
docs/               # contracts + per-item markdown
```

## Brand

**Kense** is the company (more products will follow). **KUI** is this product; use **`kui`** in crate names, CLI, modules, and paths. See [`website/public/assets/BRAND.md`](./website/public/assets/BRAND.md).

## Engineering

- [`AGENTS.md`](./AGENTS.md) — agent and contributor rules (Microsoft Pragmatic Rust Guidelines)
- [`DESIGN.md`](./DESIGN.md) — chrome tokens, Button metrics, interaction
- [`docs/PLATFORMS.md`](./docs/PLATFORMS.md) — native + WASM matrix

## License

MIT.
