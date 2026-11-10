"""Publish the preview shell, WASM, bindings and fonts as one immutable build."""
import gzip
import hashlib
from pathlib import Path
import shutil

root = Path(__file__).resolve().parents[1]
source = root / "apps/web/www"
public = root / "website/public/stories"
wasm = source / "pkg/kui_web_bg.wasm"
# A static host can serve these bytes without configuring content negotiation.
with wasm.open("rb") as binary, Path(str(wasm) + ".gz").open("wb") as output:
    with gzip.GzipFile(filename="", fileobj=output, mode="wb", compresslevel=6, mtime=0) as compressed:
        shutil.copyfileobj(binary, compressed)
files = [source / name for name in ("index.html", "preview.js", "fonts/KUIInputSans.ttf")]
files += sorted(path for path in (source / "pkg").rglob("*") if path.is_file() and not path.name.endswith(".d.ts"))
digest = hashlib.sha256()
for path in files:
    digest.update(path.relative_to(source).as_posix().encode() + b"\0")
    digest.update(hashlib.sha256(path.read_bytes()).digest())
revision = digest.hexdigest()[:20]
bundle = public / "v" / revision
for path in files:
    target = bundle / path.relative_to(source)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(path, target)

# Keep /stories/ usable as a standalone gallery, with the same versioned imports.
html = (source / "index.html").read_text()
html = html.replace('src="./preview.js"', f'src="./v/{revision}/preview.js"')
html = html.replace('href="./pkg/', f'href="./v/{revision}/pkg/')
(public / "index.html").write_text(html)
for path in (source / "fonts/KUIStorySans-OFL.txt", source / "fonts/NotoSans-LICENSE",
             root / "apps/story/assets/Inter-LICENSE", root / "apps/story/assets/FiraCode-LICENSE"):
    shutil.copyfile(path, public / path.name)
(public / "revision.txt").write_text(revision + "\n")
# Do not ship duplicate WASM binaries from earlier local builds.
for old in bundle.parent.iterdir():
    if old != bundle and old.is_dir() and len(old.name) == 20 and all(c in "0123456789abcdef" for c in old.name):
        shutil.rmtree(old)
for name in ("pkg", "fonts", "preview.js"):
    old = public / name
    if old.is_dir():
        shutil.rmtree(old)
    elif old.is_file():
        old.unlink()
