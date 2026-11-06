# KUI development tasks. See AGENTS.md for the canonical gate list.

# List available recipes.
default:
    @just --list

# Run the desktop story gallery.
# runtime_shaders compiles Metal shaders at runtime, so no Xcode/metal CLI is needed.
dev:
    cargo run -p kui-story --features native,gpui_platform/runtime_shaders

# Run the story gallery without the native window feature.
story:
    cargo run -p kui-story

# Build the WASM web gallery and serve it at http://localhost:8080
web:
    ./scripts/build-web.sh
    miniserve apps/web/www -p 8080 --index index.html

# Build the organization website, check it, and serve at http://localhost:8081
site:
    ./scripts/build-site.sh
    python3 scripts/check-site.py website/dist
    miniserve website/dist -p 8081 --index index.html

# Format the workspace.
fmt:
    cargo fmt

# Check formatting.
fmt-check:
    cargo fmt --check

# Lint the gated crates.
clippy:
    cargo clippy -p kense-kui -p kui-story -p kui-cli -p kui-build -- -D warnings

# Run story tests.
test:
    cargo test -p kui-story

# Check the library for native.
check:
    cargo check -p kense-kui

# Check the library for WASM.
check-wasm:
    cargo check -p kense-kui --target wasm32-unknown-unknown --no-default-features

# Check the WASM web host.
check-web:
    cargo check -p kui-web --target wasm32-unknown-unknown

# Rebuild the registry (JSON, docs stubs, preview hashes).
registry:
    cargo run -p kui-build -- build

# Verify story previews match registry sources.
verify-preview:
    cargo run -p kui-build -- verify-preview

# Full local gate before landing a change (AGENTS.md M-STATIC-VERIFICATION).
gate: fmt-check clippy test check-wasm check-web registry verify-preview
