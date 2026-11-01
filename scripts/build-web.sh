#!/usr/bin/env bash
# Build the KUI Button gallery for wasm32-unknown-unknown and bind JS glue.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

rustup target add wasm32-unknown-unknown >/dev/null
BINDGEN_VERSION=$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/"/,"",$3); print $3; exit}' Cargo.lock)
if ! command -v wasm-bindgen >/dev/null 2>&1 || [[ "$(wasm-bindgen --version)" != "wasm-bindgen $BINDGEN_VERSION" ]]; then
  echo "installing wasm-bindgen-cli…"
  cargo install wasm-bindgen-cli --locked --version "=$BINDGEN_VERSION"
fi

cargo build -p kui-web --target wasm32-unknown-unknown --release
OUT="$ROOT/apps/web/www/pkg"
mkdir -p "$OUT"
wasm-bindgen \
  --target web --no-typescript \
  --out-dir "$OUT" \
  "${CARGO_TARGET_DIR:-$ROOT/target}/wasm32-unknown-unknown/release/kui_web.wasm"

python3 "$ROOT/scripts/package-web-preview.py"
echo "KUI WASM gallery ready: apps/web/www/"
echo "Serve with: python3 -m http.server -d apps/web/www 8080"
