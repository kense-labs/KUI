# KUI agent rules

- Follow root `AGENTS.md` (Microsoft Pragmatic Rust Guidelines) and `DESIGN.md`.
- Prefer existing KUI registry items over new controls.
- Use `kui` CLI install commands; do not invent parallel packages named otherwise.
- Keep GPUI pinned exactly (`=0.3.10`).
- Preserve keyboard activation and disabled semantics on Button.
- Sentence case labels. No emoji in chrome.
- No tautological tests; no meta design journals in crate docs.
