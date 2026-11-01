"""Keep previous project Pages links useful after moving the site to the organization."""
from pathlib import Path
import html
import json
import shutil
import sys

site, out = map(Path, sys.argv[1:])
host = "https://kense-labs.github.io"
out.mkdir(parents=True, exist_ok=True)
for name in ("md", "r"):
    shutil.copytree(site / name, out / name, dirs_exist_ok=True)
for name in ("llms.txt", ".nojekyll"):
    shutil.copy2(site / name, out / name)
for page in site.rglob("index.html"):
    path = page.relative_to(site)
    if any(x in path.parts for x in ("assets", "stories")):
        continue
    route = "/" + path.parent.as_posix().strip("./") + "/"
    route = { "//": "/kui/", "/en/": "/en/kui/" }.get(route, route)
    target = host + route
    dest = out / path
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(f'<!doctype html><meta charset="utf-8"><title>Kense</title><link rel="canonical" href="{html.escape(target)}"><script>location.replace({json.dumps(target)}+location.search+location.hash)</script><meta http-equiv="refresh" content="0;url={html.escape(target)}"><a href="{html.escape(target)}">Kense →</a>\n')
