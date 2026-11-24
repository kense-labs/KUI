"""Build KUI multi-weight bilingual & Latin preview fonts from OFL sources.

Usage:
    python scripts/subset-story-font.py NotoSansSC[wght].ttf [InterVariable.ttf] [NotoSansSymbols2-Regular.ttf]
"""

from __future__ import annotations

from pathlib import Path
import sys
import tempfile

from fontTools import subset
from fontTools.merge import Merger
from fontTools.ttLib import TTFont
from fontTools.ttLib.scaleUpem import scale_upem
from fontTools.varLib.instancer import instantiateVariableFont

ROOT = Path(__file__).resolve().parents[1]
KEEP_TRUETYPE_TABLES = [
    "head",
    "hhea",
    "maxp",
    "OS/2",
    "hmtx",
    "cmap",
    "fpgm",
    "prep",
    "cvt ",
    "loca",
    "glyf",
    "name",
    "post",
]


def set_names_and_weight(font: TTFont, family: str, style: str, weight: int) -> None:
    if "STAT" in font:
        del font["STAT"]
    ps_family = family.replace(" ", "")
    full_name = family if style == "Regular" else f"{family} {style}"
    ps_name = f"{ps_family}-{style}"
    names = {
        1: family,
        2: style,
        3: ps_name,
        4: full_name,
        6: ps_name,
        16: family,
        17: style,
    }
    name_table = font["name"]
    name_table.names = [r for r in name_table.names if r.nameID < 256]
    for name_id, value in names.items():
        name_table.setName(value, name_id, 3, 1, 0x0409)

    os2 = font["OS/2"]
    os2.usWeightClass = weight
    # Clear REGULAR (bit 6) and BOLD (bit 5), then set appropriately.
    os2.fsSelection &= ~((1 << 6) | (1 << 5))
    if weight == 400:
        os2.fsSelection |= 1 << 6
    elif weight >= 700:
        os2.fsSelection |= 1 << 5

    if weight >= 700:
        font["head"].macStyle |= 1
    else:
        font["head"].macStyle &= ~1


def strip_to_truetype(font: TTFont) -> None:
    for tag in list(font.keys()):
        if tag != "GlyphOrder" and tag not in KEEP_TRUETYPE_TABLES:
            del font[tag]


def is_cjk_or_fullwidth(cp: int) -> bool:
    return (
        0x2E80 <= cp <= 0x9FFF
        or 0xF900 <= cp <= 0xFAFF
        or 0xFE30 <= cp <= 0xFE6F
        or 0xFF00 <= cp <= 0xFFEF
        or 0x20000 <= cp <= 0x323AF
    )


def build_latin_family(inter_path: Path, used_chars: set[int]) -> None:
    base_lr_path = ROOT / "apps/story/assets/LatinRegular.ttf"
    existing_latin = set()
    if base_lr_path.exists():
        existing_latin = set(TTFont(base_lr_path).getBestCmap().keys())

    probe = TTFont(inter_path, recalcTimestamp=False)
    inter_cmap = set(probe.getBestCmap().keys())
    latin_chars = (existing_latin | (used_chars & inter_cmap)) - {
        cp for cp in used_chars if is_cjk_or_fullwidth(cp)
    }

    specs = [
        ("Regular", 400, "LatinRegular.ttf"),
        ("Medium", 500, "LatinMedium.ttf"),
        ("SemiBold", 600, "LatinSemiBold.ttf"),
        ("Bold", 700, "LatinBold.ttf"),
    ]
    for style, weight, filename in specs:
        font = TTFont(inter_path, recalcTimestamp=False)
        opts = subset.Options()
        opts.layout_features = ["kern", "liga", "calt", "tnum", "cv05", "ss01", "ss03"]
        opts.name_IDs = ["*"]
        sub = subset.Subsetter(options=opts)
        sub.populate(unicodes=latin_chars)
        sub.subset(font)
        if "fvar" in font:
            axes = {a.axisTag: a.defaultValue for a in font["fvar"].axes}
            axes["wght"] = float(weight)
            if "opsz" in axes:
                axes["opsz"] = 14.0
            font = instantiateVariableFont(font, axes, inplace=True)
        set_names_and_weight(font, "KUI Preview Latin", style, weight)
        font.save(ROOT / "apps/story/assets" / filename)


def build_bilingual_face(
    sc_path: Path,
    inter_path: Path | None,
    sym_path: Path | None,
    target_chars: set[int],
    family: str,
    style: str,
    weight: int,
    out_path: Path,
) -> None:
    sc_probe = TTFont(sc_path, recalcTimestamp=False)
    sc_cmap = set(sc_probe.getBestCmap().keys())
    missing_cjk = sorted(
        c
        for c in target_chars
        if (0x3400 <= c <= 0x9FFF or 0xF900 <= c <= 0xFAFF or 0x20000 <= c <= 0x323AF)
        and c not in sc_cmap
    )
    if missing_cjk:
        raise ValueError("Missing CJK glyphs: " + "".join(map(chr, missing_cjk)))

    native_ascent = sc_probe["hhea"].ascent
    native_descent = sc_probe["hhea"].descent
    native_line_gap = sc_probe["hhea"].lineGap
    typo_asc = sc_probe["OS/2"].sTypoAscender
    typo_desc = sc_probe["OS/2"].sTypoDescender
    typo_gap = sc_probe["OS/2"].sTypoLineGap
    win_asc = sc_probe["OS/2"].usWinAscent
    win_desc = sc_probe["OS/2"].usWinDescent

    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        part_paths: list[str] = []
        claimed: set[int] = set()

        if inter_path is not None and inter_path.exists():
            iv = TTFont(inter_path, recalcTimestamp=False)
            iv_cmap = set(iv.getBestCmap().keys())
            iv_chars = {
                c for c in target_chars if c in iv_cmap and not is_cjk_or_fullwidth(c)
            }
            if iv_chars:
                sub = subset.Subsetter()
                sub.populate(unicodes=iv_chars)
                sub.subset(iv)
                if "fvar" in iv:
                    axes = {a.axisTag: a.defaultValue for a in iv["fvar"].axes}
                    axes["wght"] = float(weight)
                    if "opsz" in axes:
                        axes["opsz"] = 14.0
                    iv = instantiateVariableFont(iv, axes, inplace=True)
                if iv["head"].unitsPerEm != 1000:
                    scale_upem(iv, 1000)
                strip_to_truetype(iv)
                p_iv = tmp / "part_inter.ttf"
                iv.save(p_iv)
                part_paths.append(str(p_iv))
                claimed |= iv_chars

        sc = TTFont(sc_path, recalcTimestamp=False)
        sc_chars = (target_chars & sc_cmap) - claimed
        # Ensure 'm' exists if Inter wasn't supplied.
        if ord("m") not in claimed:
            sc_chars.add(ord("m"))
        sub_sc = subset.Subsetter()
        sub_sc.populate(unicodes=sc_chars)
        sub_sc.subset(sc)
        if "fvar" in sc:
            sc = instantiateVariableFont(sc, {"wght": float(weight)}, inplace=True)
        strip_to_truetype(sc)
        p_sc = tmp / "part_sc.ttf"
        sc.save(p_sc)
        part_paths.append(str(p_sc))
        claimed |= sc_chars

        if sym_path is not None and sym_path.exists():
            sym = TTFont(sym_path, recalcTimestamp=False)
            sym_cmap = set(sym.getBestCmap().keys())
            # Pull geometric triangles / UI arrows missing from both Inter and NotoSansSC.
            extra_ui_syms = {0x25B8, 0x25BE, 0x25B4, 0x25C2}
            sym_chars = ((target_chars | extra_ui_syms) & sym_cmap) - claimed
            # Restrict to BMP geometric/arrow ranges so dingbats/astrological glyphs aren't bloated in.
            sym_chars = {c for c in sym_chars if 0x2190 <= c <= 0x25FF}
            if sym_chars:
                sub_sym = subset.Subsetter()
                sub_sym.populate(unicodes=sym_chars)
                sub_sym.subset(sym)
                if "fvar" in sym:
                    sym = instantiateVariableFont(
                        sym, {"wght": float(min(weight, 600))}, inplace=True
                    )
                if sym["head"].unitsPerEm != 1000:
                    scale_upem(sym, 1000)
                strip_to_truetype(sym)
                p_sym = tmp / "part_sym.ttf"
                sym.save(p_sym)
                part_paths.append(str(p_sym))

        merged = Merger().merge(part_paths) if len(part_paths) > 1 else TTFont(part_paths[0])
        merged["hhea"].ascent = native_ascent
        merged["hhea"].descent = native_descent
        merged["hhea"].lineGap = native_line_gap
        merged["OS/2"].sTypoAscender = typo_asc
        merged["OS/2"].sTypoDescender = typo_desc
        merged["OS/2"].sTypoLineGap = typo_gap
        merged["OS/2"].usWinAscent = win_asc
        merged["OS/2"].usWinDescent = win_desc
        merged["OS/2"].sCapHeight = 728
        merged["OS/2"].sxHeight = 546
        set_names_and_weight(merged, family, style, weight)
        merged.save(out_path)


def main() -> None:
    sc_path = Path(sys.argv[1])
    inter_path = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    sym_path = Path(sys.argv[3]) if len(sys.argv) > 3 else None

    sources = [*ROOT.glob("apps/story/src/**/*.rs"), *ROOT.glob("registry/src/**/*.rs")]
    chars = set(range(32, 127)) | {
        ord(c) for p in sources for c in p.read_text() if ord(c) >= 128
    }

    if inter_path is not None and inter_path.exists():
        build_latin_family(inter_path, chars)

    fonts_dir = ROOT / "apps/web/www/fonts"
    for style, weight, filename in [
        ("Regular", 400, "KUIStorySans.ttf"),
        ("Medium", 500, "KUIStorySans-Medium.ttf"),
        ("SemiBold", 600, "KUIStorySans-SemiBold.ttf"),
    ]:
        build_bilingual_face(
            sc_path,
            inter_path,
            sym_path,
            chars,
            "KUI Story Sans",
            style,
            weight,
            fonts_dir / filename,
        )

    # Editable demos also render user input that never appears in gallery copy.
    input_chars = set(chars)
    for lead in range(0xB0, 0xF8):
        for trail in range(0xA1, 0xFF):
            input_chars.update(
                map(ord, bytes([lead, trail]).decode("gb2312", errors="ignore"))
            )
    build_bilingual_face(
        sc_path,
        inter_path,
        sym_path,
        input_chars,
        "KUI Input Sans",
        "Regular",
        400,
        fonts_dir / "KUIInputSans.ttf",
    )


if __name__ == "__main__":
    main()
