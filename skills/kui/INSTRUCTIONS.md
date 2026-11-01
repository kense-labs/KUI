# KUI agent rules

- Follow root `AGENTS.md` (Microsoft Pragmatic Rust Guidelines) and `DESIGN.md`.
- Prefer existing KUI controls over new controls.
- The component package is `kense-kui`; install with `cargo add kense-kui --rename kui --no-default-features` and import `kui::`.
- To own source, install `kui-cli` from GitHub, run `kui init` and `kui add <id>`, and import `crate::ui::`. The CLI is not published to crates.io.
- Keep GPUI pinned exactly (`=0.3.10`).
- Preserve keyboard activation and disabled semantics on Button.
- Sentence case labels. No emoji in chrome.
- No tautological tests; no meta design journals in crate docs.
