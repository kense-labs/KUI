# AGENTS.md

Operating rules for humans and coding agents working in **KUI**.

**Kense** is the company; **KUI** is one Kense product (others will exist outside this repo). The component package is **`kense-kui`** on crates.io; its Rust library and workspace dependency are **`kui`**. CLI, modules, paths, and registry ids use **`kui`**. Do not reintroduce Arc, gpui-kit, a3s, or tenant-kit branding. Do not invent sibling product names here. License is **MIT**.

Default chrome is pure black/white. Kense brand `#171717` is for important brand moments only (logo lockup, org attribution)—not everyday fills or primary buttons.

## Authority

Rust code in this repository follows Microsoft **[Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/)**, plus the upstream:

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)

When a rule below cites an `M-*` or `C-*` id, that id is the source of truth. Prefer the condensed checklist at <https://microsoft.github.io/rust-guidelines/guidelines/checklist/> when reviewing a change.

Visual chrome is specified only in [`DESIGN.md`](./DESIGN.md). Brand naming is also summarized in [`website/public/assets/BRAND.md`](./website/public/assets/BRAND.md).

## Repository map

| Path | Role |
| --- | --- |
| `registry/` | Authoritative UI source; package `kense-kui`, library `kui`. Preview and install share this tree. |
| `apps/story/` | Shared gallery lib + desktop/smoke host (`kui-story`). |
| `apps/web/` | WASM web host (`kui-web`) for the same gallery. |
| `tools/cli/` | `kui` binary: `init`, `add`, `doctor`, `sync-mod`. |
| `tools/registry-build/` | `kui-build`: registry JSON, docs stubs, preview hashes. |
| `docs/` | Contracts and per-item Markdown. See `docs/PLATFORMS.md`. |
| `skills/kui/` | Agent skills for install and Button usage. |
| `templates/app/` | Blank app scaffold for `kui init`. |
| `website/public/r/` | Generated registry items. |
| `website/site/` | Rust HTML generator for the Kense homepage and the bilingual KUI product. |
| `website/client/` | Rust/WASM document interactions; component navigation reuses one live GPUI frame. |

The website and CLI expose the same 108 installable components in catalog order. Foundation is a shared dependency. The catalog in tools/catalog is authoritative for installable items.

## Platforms

KUI targets **native desktop** and **WASM web** with one control source. Details: [`docs/PLATFORMS.md`](./docs/PLATFORMS.md).

- Library: `cargo check -p kense-kui --target wasm32-unknown-unknown --no-default-features` must stay green.
- Do not enable `kui/test-support` on wasm32.
- Do not fork chrome per platform. Use `cfg(target_family = "wasm")` only for host-only OS APIs.
- Desktop story: `cargo run -p kui-story --features native`
- Web story: `./scripts/build-web.sh`
- Organization website: `./scripts/build-site.sh`, then `python3 scripts/check-site.py website/dist`. Keep document controls in the KUI HTML backend and previews on GPUI.

## Toolchain and verification (M-STATIC-VERIFICATION)

- Edition: `2024` (workspace).
- GPUI pin: `gpui-pre` / `gpui-pre-platform` / `gpui-pre-web` at `=0.3.10` exact. Never widen to a caret range.
- Default features use `gpui` with `default-features = false` and `test-support` so host CI works without Metal shaders.
- Required local gates before landing:

```bash
cargo fmt --check
cargo clippy -p kense-kui -p kui-story -p kui-cli -p kui-build -- -D warnings
cargo test -p kui-story
cargo check -p kense-kui --target wasm32-unknown-unknown --no-default-features
cargo check -p kui-web --target wasm32-unknown-unknown
cargo run -p kui-build -- build
cargo run -p kui-build -- verify-preview
```

- Prefer `cargo audit` when dependencies change.
- Do not introduce `unsafe` without an `# Safety` section and a clear reason. Prefer safe GPUI APIs.

## Architecture

1. **Library and copy-source distribution.** Applications depend on the `kense-kui` package as `kui`, or own selected `ui/` source installed by the CLI. Both distribution paths, story, and CI use the same `registry/` tree. Only `kense-kui` is published to crates.io.
2. **Dual runtime, single UI.** Native and WASM hosts bootstrap GPUI differently; gallery and registry controls stay shared.
3. **Behavior vs presentation.** `registry/src/primitives` owns behavior (ids, disabled, sizing enums). `registry/src/components` owns chrome (tokens, padding, variants). Do not put brand colors in primitives.
4. **Foundation once.** `kui-foundation` installs theme, motion, and `init`. Components must not reinstall foundation by default.
5. **Preview = source.** `apps/story` must keep a path dependency on `kui`. `kui-build verify-preview` enforces this.
6. **Stable `ElementId`.** Changing an id resets focus and interaction state. Treat ids as part of the public contract.

## Public API UX (Library / UX)

Apply these Microsoft rules to every public item in `registry/`:

- **M-SIMPLE-ABSTRACTIONS** — Do not nest complex generics in the Button (or future control) public surface.
- **M-AVOID-WRAPPERS** — Do not expose `Rc` / `Arc` / `RefCell` in public constructors. Keep them inside `on_click` wiring if needed.
- **M-DI-HIERARCHY** — Prefer concrete types over generics over `dyn Trait` for app-facing APIs.
- **M-ERRORS-CANONICAL-STRUCTS** — CLI and build tools use `anyhow` at the binary edge; library failures that become public should be dedicated error structs, not stringly errors.
- **C-CONV / C-GETTER** — Conversions use `as_` / `to_` / `into_`; getters follow Rust convention (`is_` / `has_` for booleans).
- **C-COMMON-TRAITS** — Public value types implement the common traits they can (`Debug`, `Clone`, `Default`, `PartialEq` where meaningful).
- **M-SINGLE-ITEM-PATH** — Each public item has one user-facing path. Do not re-export the same type under multiple modules “for convenience.”

Builder methods on controls stay chainable and consume `self` by value unless GPUI requires otherwise.

## Documentation (M-MODULE-DOCS, M-CANONICAL-DOCS, M-FIRST-DOC-SENTENCE)

- Every public module has `//!` docs. First sentence ≤ ~15 words; it becomes the rustdoc summary.
- Public items use canonical sections when applicable: summary, extended prose, `# Examples`, `# Errors`, `# Panics`, `# Safety`, `# Abort`.
- Document the end state, not the design journey (**M-NO-META-DESIGN-DOCUMENTATION**). No “we chose X over Y” essays in crate docs.
- Keep `docs/items/<id>.md` in sync when changing installable API or behavior. Run `kui-build docs` only to create missing stubs; prefer editing authored Markdown.

## AI-oriented rules (M-DESIGN-FOR-AI)

- Reuse controls from `kense-kui` or install with `kui add <id>` instead of inventing a parallel control. Keep `kui::` imports for the library and `crate::ui::` imports for copied source.
- Use strong types (`ButtonVariant`, `Size`, `ThemeMode`) instead of magic strings or bool pairs (**C-NEWTYPE** spirit).
- Make behavior testable under `gpui::TestApp` / `test-support` without a native window.
- Do not write tautological tests that only restate constants (**M-TAUTOLOGICAL-TESTS**).
- Write idiomatic Rust, not ports of React/C# patterns (**M-RUST-SHAPED**). GPUI `Render` / `IntoElement` / controlled callbacks are the native shapes.

## Naming

| Kind | Convention |
| --- | --- |
| Product | KUI |
| Crates | `kense-kui` (lib `kui`), `kui-story`, `kui-cli` (bin `kui`), `kui-build`, `kui-template` |
| Registry namespace | `@kui` |
| Install ids | `kui-foundation`, `button` |
| Modules | `snake_case` |
| Types / enums | `PascalCase` |
| Builders | `with_*` when taking `self` and setting a field; short verbs (`primary`, `label`) when they read as fluent product API |

## Error handling and panics

- Library code: no `unwrap` / `expect` on fallible I/O or user input. Prefer `Result` and typed errors.
- `expect` only for programmer-invariants that are impossible after a prior check, with a message that states the invariant.
- Story / tests may use `expect` at window open boundaries with a clear label.

## Testing

- Unit tests cover observable Button behavior (variants build under theme, disabled/loading block intent where modeled).
- Prefer `TestApp` smoke over requiring Metal/Xcode in CI.
- After registry file moves, re-run `kui-build verify-preview`.

## Delivery

- Keep changes scoped to the requested control or tool. Do not expand the MVP catalog casually.
- Publish only `kense-kui`; follow [`docs/PUBLISHING.md`](./docs/PUBLISHING.md) for packaging, organization ownership, and documentation checks.
- Stage files deliberately. Do not commit secrets, `.env`, or unrelated worktrees.
- Commits follow [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): summary` (e.g. `feat`, `fix`, `chore`, `docs`, `refactor`). Subjects are imperative and say why when useful.
- Update `DESIGN.md` in the same change when chrome tokens, Button metrics, or interaction states change.

## Quick commands

```bash
cargo check -p kense-kui
cargo check -p kense-kui --target wasm32-unknown-unknown --no-default-features
cargo run -p kui-story
cargo run -p kui-story --features native
./scripts/build-web.sh
cargo install --path tools/cli
kui init
kui add button
kui doctor
```
