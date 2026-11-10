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
    run("python3", str(ROOT / "scripts/check-site-inputs.py"), cwd=ROOT)
    run("cargo", "build", "-p", "kui-cli", cwd=ROOT)
    catalog = json.loads((ROOT / "website/public/r/catalog.json").read_text())
    items = [item["name"] for item in catalog if item["kind"] == "component"]
    site = ROOT / "website/site"
    files = re.findall(r'include_str!\("\.\./docs/([^"/]+\.json)"\)', (site / "src/docs.rs").read_text())
    documents = []
    for filename in files:
        data = json.loads((site / "docs" / filename).read_text())
        if isinstance(data, list):
            documents.extend(row["id"] for row in data)
        elif "id" in data:
            documents.append(data["id"])
    assert items == documents, "Rebuild the generated registry to match the site documents."
    manifests = [*items, "all-components"]
    (work / "Cargo.toml").write_text('[workspace]\nresolver = "2"\nmembers = ' + json.dumps(manifests))
    shutil.copyfile(ROOT / "Cargo.lock", work / "Cargo.lock")
    for index, name in enumerate(manifests):
        app = work / name
        app.mkdir()
        (app / "Cargo.toml").write_text(f'[package]\nname = "{name}"\nversion = "0.0.0"\nedition = "2024"\n[lib]\npath = "lib.rs"\n')
        run(BINARY, "init", "--path", app, "--registry", ROOT / "registry", cwd=work)
        if name == "all-components":
            (app / "ui/foundation/mod.rs").write_text("pub mod init;\npub mod motion;\npub mod theme;\npub mod tokens;\n")
        ids = items if name == "all-components" else [name]
        # Exercise both registry entry points: packaged JSON and local source.
        run(BINARY, "add", *ids, "--path", app, cwd=ROOT if index % 2 == 0 else work)
        run(BINARY, "sync-mod", "--path", app, cwd=work)
        code = ['pub mod ui;']
        for item in ids:
            if item == "button":
                code.append('pub fn mount_button(cx: &gpui::App) { let _ = ui::components::button::Button::new("save").secondary().label("Save").with_icon(ui::primitives::glyph::Glyph::Check, false).loading(true).into_themed(cx); }')
                continue
            doc = (ROOT / f"docs/items/{item}.md").read_text()
            snippet = re.search(r'```rust\n(.*?)\n```', doc, re.S).group(1).replace("kui::", "crate::ui::")
            code.append(f'#[allow(unused_variables)]\npub fn mount_{item.replace("-", "_")}(window: &mut gpui::Window, cx: &mut gpui::App) {{\n{snippet}\n}}')
        (app / "lib.rs").write_text('\n'.join(code))
    for target in ([], ["--target", "wasm32-unknown-unknown"]):
        run("cargo", "check", "--workspace", *target, cwd=work)


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="kui-installs-") as tmp:
        verify(Path(tmp))
