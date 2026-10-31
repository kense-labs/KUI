#!/usr/bin/env bash
# Build the KUI Button gallery for wasm32-unknown-unknown and bind JS glue.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

rustup target add wasm32-unknown-unknown >/dev/null
if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "installing wasm-bindgen-cli…"
  cargo install wasm-bindgen-cli --locked
fi

cargo build -p kui-story-web --target wasm32-unknown-unknown --release
OUT="$ROOT/apps/story-web/www/pkg"
mkdir -p "$OUT"
wasm-bindgen \
  --target web \
  --out-dir "$OUT" \
  "$ROOT/target/wasm32-unknown-unknown/release/kui_story_web.wasm"

echo "KUI WASM gallery ready: apps/story-web/www/"
echo "Serve with: python3 -m http.server -d apps/story-web/www 8080"
