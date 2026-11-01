#!/usr/bin/env bash
# Build the organization website without a Node, Astro or TypeScript toolchain.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
./scripts/build-web.sh
staging="$(mktemp -d "${TMPDIR:-/tmp}/kui-site.XXXXXXXX")"
trap 'rm -rf "$staging"' EXIT
cargo build --locked --release --target wasm32-unknown-unknown -p kui-site-client
wasm-bindgen --target web --no-typescript --out-name site --out-dir "$staging/client" \
  "${CARGO_TARGET_DIR:-$root/target}/wasm32-unknown-unknown/release/kui_site_client.wasm"
KUI_RELEASE="${KUI_RELEASE:-$(git rev-parse HEAD)}" cargo run --locked -p kui-site -- "$staging/site" "$staging/client"
mkdir -p website/dist
rsync -a --delete "$staging/site/" website/dist/
