"""Build the preview's bilingual OFL font: python subset-story-font.py NotoSansSC[wght].ttf"""
import sys
from pathlib import Path
from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

root = Path(__file__).resolve().parents[1]
faces = [*root.glob("apps/story/src/**/*.rs"), *root.glob("registry/src/**/*.rs")]
chars = set(range(32, 127)) | {ord(c) for p in faces for c in p.read_text() if ord(c) >= 128}


def build(family, chars):
    font = TTFont(sys.argv[1], recalcTimestamp=False)
    missing = sorted(c for c in chars if (0x3400 <= c <= 0x9fff or 0xf900 <= c <= 0xfaff or 0x20000 <= c <= 0x323af) and c not in font.getBestCmap())
    if missing:
        raise ValueError("Missing CJK glyphs: " + "".join(map(chr, missing)))
    sub = subset.Subsetter()
    sub.populate(unicodes=chars)
    sub.subset(font)
    if "fvar" in font:
        font = instantiateVariableFont(font, {"wght": 400}, inplace=True)
    names = {1:family, 2:"Regular", 3:family+" Regular", 4:family+" Regular", 6:family.replace(" ", "")+"-Regular", 16:family, 17:"Regular"}
    for record in font["name"].names:
        if record.nameID in names:
            record.string = names[record.nameID].encode(record.getEncoding())
    font.save(root / "apps/web/www/fonts" / (family.replace(" ", "")+".ttf"))


build("KUI Story Sans", chars)
# The editable demos must also render user input that never appears in the gallery copy.
for lead in range(0xb0, 0xf8):
    for trail in range(0xa1, 0xff):
        chars.update(map(ord, bytes([lead, trail]).decode("gb2312", errors="ignore")))
build("KUI Input Sans", chars)
