"""Compile the documented copy-source examples in otherwise empty consumer crates."""
from pathlib import Path
import json
import os
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
BINARY = TARGET / "debug" / ("kui.exe" if os.name == "nt" else "kui")


def run(*args, cwd):
    subprocess.run(args, cwd=cwd, check=True)


def verify(work):
    run("cargo", "build", "-p", "kui-cli", cwd=ROOT)
    catalog = json.loads((ROOT / "website/public/r/catalog.json").read_text())
    items = [item["name"] for item in catalog if item["kind"] == "component" and item["name"] != "button"]
    assert ["button", *items] == [entry["name"] for entry in catalog if entry["kind"] == "component"] == [
        "button", "action-button", "split-button", "button-group", "floating-button-group",
        "expanding-button-group", "copy-button", "action-swap", "icon-morph", "confirm-morph", "theme-switch",
        "hold-to-confirm", "swipe-actions", "reactions", "dropdown-menu", "context-menu", "user-menu",
        "input", "textarea", "password-field", "password-strength", "search-field", "expanding-search", "inline-edit",
    ]
    manifests = ["button", *items, "all-components"]
    (work / "Cargo.toml").write_text('[workspace]\nresolver = "2"\nmembers = ' + json.dumps(manifests))
    shutil.copyfile(ROOT / "Cargo.lock", work / "Cargo.lock")
    for index, name in enumerate(manifests):
        app = work / name
        app.mkdir()
        (app / "Cargo.toml").write_text(f'[package]\nname = "{name}"\nversion = "0.0.0"\nedition = "2024"\n[lib]\npath = "lib.rs"\n')
        run(BINARY, "init", "--path", app, "--registry", ROOT / "registry", cwd=work)
        if name == "all-components":
            (app / "ui/foundation/mod.rs").write_text("pub mod init;\npub mod motion;\npub mod theme;\npub mod tokens;\n")
        ids = ["button", *items] if name == "all-components" else [name]
        # Exercise both registry entry points: packaged JSON and local source.
        run(BINARY, "add", *ids, "--path", app, cwd=ROOT if index % 2 == 0 else work)
        run(BINARY, "sync-mod", "--path", app, cwd=work)
        code = ['pub mod ui;']
        for item in ids:
            if item == "button":
                code.append('pub fn mount_button(cx: &gpui::App) { let _ = ui::components::button::Button::new("save").secondary().label("Save").with_icon(ui::primitives::glyph::Glyph::Check, false).loading(true).into_themed(cx); }')
                continue
            doc = (ROOT / f"docs/items/{item}.md").read_text()
            snippet = re.search(r'```rust\n(.*?)\n```', doc, re.S).group(1)
            code.append(f'pub fn mount_{item.replace("-", "_")}(#[allow(unused_variables)] window: &mut gpui::Window, cx: &mut gpui::App) {{\n{snippet}\nlet _ = control;\n}}')
        (app / "lib.rs").write_text('\n'.join(code))
    for target in ([], ["--target", "wasm32-unknown-unknown"]):
        run("cargo", "check", "--workspace", *target, cwd=work)


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="kui-installs-") as tmp:
        verify(Path(tmp))
