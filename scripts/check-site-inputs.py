"""Check documentation inputs before the expensive Web and site builds."""

import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "website/site/docs"
source = (ROOT / "website/site/src/docs.rs").read_text()
group_block = re.search(r"const GROUPS:.*?=\s*&\[(.*?)\];", source, re.S)
if group_block is None:
    raise SystemExit("Could not find documentation groups")
groups = set(re.findall(r'"([^"\n]+)"', group_block.group(1)))
labels = json.loads((DOCS / "labels.json").read_text())
files = re.findall(r'include_str!\("\.\./docs/([^"/]+\.json)"\)', source)
entries = []
for name in files:
    data = json.loads((DOCS / name).read_text())
    if isinstance(data, dict) and "id" in data:
        entries.append(data)
    elif isinstance(data, list):
        entries.extend(data)

seen = set()
for entry in entries:
    identifier = entry.get("id")
    if not isinstance(identifier, str) or not identifier or identifier in seen:
        raise SystemExit(f"Missing or duplicate component ID: {identifier!r}")
    seen.add(identifier)
    group = entry.get("group") or "buttons"
    key = f"sidebar.group.{group}"
    if group not in groups or not all(labels.get(key, {}).get(lang) for lang in ("en", "zh")):
        raise SystemExit(f"Unknown or untranslated group {group!r} for {identifier}")

if not entries:
    raise SystemExit("No component documentation found")

# The site, installer and registry are generated from the same ordered list.
# Catch missing or duplicate entries before compiling the Web preview.
catalog_source = (ROOT / "tools/catalog/src/lib.rs").read_text()
catalog_block = re.search(r"pub const ITEMS:.*?=\s*&\[(.*?)\];", catalog_source, re.S)
if catalog_block is None:
    raise SystemExit("Could not find the base catalog")
catalog_ids = re.findall(r'\(\s*"([^"\n]+)"\s*,', catalog_block.group(1))
library_source = (ROOT / "tools/catalog/src/library.rs").read_text()
catalog_ids.extend(re.findall(r'\bid:\s*"([^"\n]+)"', library_source))
catalog_ids = [identifier for identifier in catalog_ids if identifier != "kui-foundation"]
doc_ids = [entry["id"] for entry in entries]
if catalog_ids != doc_ids:
    missing_docs = sorted(set(catalog_ids) - set(doc_ids))
    missing_catalog = sorted(set(doc_ids) - set(catalog_ids))
    raise SystemExit(
        f"Catalog and documentation differ: missing docs={missing_docs}, "
        f"missing catalog={missing_catalog}; check order if both are empty"
    )

# New examples use a shared GPUI host. A document without a registered story
# would build successfully but leave the visitor looking at the previous demo.
extra_ids = {
    entry["id"]
    for name in files
    if name.endswith("-extra.json")
    for entry in json.loads((DOCS / name).read_text())
}
stories = ROOT / "apps/story/src/extra_story"
dispatcher = (stories / "mod.rs").read_text()
modules = set(re.findall(r"\b(\w+)::story\(id, window, cx\)", dispatcher))
registered = set()
owners = {}


def register(identifier, module):
    if identifier in owners:
        raise SystemExit(f"Preview {identifier!r} is registered in both {owners[identifier]!r} and {module!r}")
    owners[identifier] = module
    registered.add(identifier)


for module in modules:
    story_source = (stories / f"{module}.rs").read_text()
    match = re.search(r"pub const IDS:.*?=\s*&\[(.*?)\];", story_source, re.S)
    if match:
        if not re.search(rf"\b{re.escape(module)}::IDS\b", dispatcher):
            raise SystemExit(f"Story module {module!r} is not included in the ID list")
        for identifier in re.findall(r'"([^"\n]+)"', match.group(1)):
            register(identifier, module)
    else:
        # Some single-example modules use an inline ID in both places.
        single = re.findall(r'id\s*==\s*"([^"\n]+)"', story_source)
        for identifier in single:
            if f'"{identifier}"' in dispatcher:
                register(identifier, module)
if registered != extra_ids:
    raise SystemExit(
        f"Extra preview registrations differ: missing stories={sorted(extra_ids - registered)}, "
        f"missing docs={sorted(registered - extra_ids)}"
    )
print(f"Checked {len(entries)} documentation entries, catalog order and {len(registered)} extra previews.")
