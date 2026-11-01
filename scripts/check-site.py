"""Validate canonical static routes, Rust document fragments, and deployable assets."""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit, unquote
import json
import sys

root = Path(sys.argv[1])
class Page(HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.elements = []
        self.feed(text)
    def handle_starttag(self, tag, attrs):
        self.elements.append((tag, dict(attrs)))
seen = set()
broken = []
def check(url, page):
    u = urlsplit(url)
    if not u.path.startswith("/") or u.scheme or u.netloc or u.path in seen:
        return
    seen.add(u.path)
    target = root / unquote(u.path).lstrip("/")
    if not target.is_file() and not (target / "index.html").is_file():
        broken.append((str(page),url))
catalog = json.loads((root / 'r/catalog.json').read_text())
ids = {item['name'] for item in catalog if item['name'] != 'kui-foundation'}
for en in (False, True):
    prefix = 'en/' if en else ''
    for sub in ('','kui','kui/docs','kui/docs/installation','kui/ai',*[f'kui/components/{id}' for id in ids]):
        path = root / prefix / sub / 'index.html'
        page = Page(path.read_text())
        canonical = next(a['href'] for t, a in page.elements if t == 'link' and a.get('rel') == 'canonical')
        expected = '/' + path.relative_to(root).as_posix().removesuffix('index.html')
        assert canonical == 'https://kense-labs.github.io' + expected, (path, canonical)
        for tag, attrs in page.elements:
            for key in ('src','href','data-src','data-preview-src'):
                if attrs.get(key): check(attrs[key],path)
        if '/components/' not in sub: continue
        id = sub.rsplit('/',1)[-1]
        stamp = next(a['content'] for t,a in page.elements if t=='meta' and a.get('name')=='kui-pages')
        slot = next(a for t,a in page.elements if t=='iframe' and a.get('data-story-key')==id)
        assert slot['src'].startswith('/stories/v/'), path
        lang = 'en' if en else 'zh'
        fragment=json.loads((root/f'assets/pages/{stamp}/{lang}/{id}.json').read_text())
        assert fragment['lang']==lang and fragment['id']==id and 'data-preview-slot' in fragment['html'], path
        assert any(a.get('data-kui-control')=='button' for t,a in page.elements), path
assert not broken, broken[:20]
assert not list(root.rglob('*.ts')), "unexpected TypeScript in release"
assert not (root / '_astro').exists(), "old frontend build present"
print(f'Verified both locales, {len(ids)} components, install registry, and {len(seen)} static destinations.')
