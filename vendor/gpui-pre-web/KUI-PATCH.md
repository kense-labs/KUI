# Web viewport geometry patch

Upstream: `gpui-pre-web` 0.3.10, Zed revision `08cbb88a152c3751a43e853425a70e9f631a5918`.
The upstream Apache-2.0 license is retained in `LICENSE-APACHE`.

This patch validates `devicePixelContentBoxSize` against CSS content bounds and
`devicePixelRatio`. Inconsistent or unavailable pixel boxes use the existing CSS
fallback. This keeps logical layout and pointer coordinates in the same space
under browser viewport emulation, without changing component metrics or the
pinned GPUI version. Consistent device boxes retain the upstream pixel alignment.

Only `src/window.rs`, the small `src/viewport_metrics.rs` helper and its test
module declaration in `src/gpui_web.rs` differ from the published crate.
Remove this patch when the pinned upstream version passes the same regression
scenarios and the browser resize check.
