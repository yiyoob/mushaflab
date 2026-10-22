"""Spike S2 (T0.4): codepoint inventory and full-text shaping audit per font.

    uv run --directory data python -m spikes.s2_fonts > docs/font-coverage.md

For every (text, font) pair, every ayah is shaped with HarfBuzz and three failure kinds
are counted (CLAUDE.md rule 6):
  * .notdef: glyph id 0 in the output (the font has no glyph for an input codepoint);
  * dotted circle: U+25CC inserted by HarfBuzz for a mark that has no base;
  * unattached mark: a GDEF mark glyph whose position is identical with the `mark` and
    `mkmk` features switched off, i.e. no anchor attachment placed it.
"""

import sys
import unicodedata
from collections import Counter
from pathlib import Path

import uharfbuzz as hb
from fontTools.ttLib import TTFont

from spikes.common import (
    extracted,
    kfgqpc_official,
    load_json,
    only,
    qul_layout,
    verified_sources,
)
from spikes.s1_editions import dk_ayahs, dk_text_pages, kfgqpc_words, skeleton

RIWAYAT = ["hafs", "shuba", "warsh", "qalun", "bazzi", "qunbul", "duri_abuamr", "susi"]
OFFICIAL_ZIP = {
    "hafs": "kfgqpc-official-hafs",
    "shuba": "kfgqpc-official-shuba",
    "warsh": "kfgqpc-official-warsh",
    "qalun": "kfgqpc-official-qalun",
    "qunbul": "kfgqpc-official-qunbul",
    "duri_abuamr": "kfgqpc-official-duri_abuamr",
    "susi": "kfgqpc-official-susi",
}
DOTTED_CIRCLE = 0x25CC
MARK_CLASS = 3
NO_MARK_FEATURES = {"mark": False, "mkmk": False}
TOP = 6


def out(line: str = "") -> None:
    print(line)


class Font:
    def __init__(self, name: str, path: Path) -> None:
        self.name = name
        self.path = path
        blob = hb.Blob.from_file_path(str(path))
        self.hb_font = hb.Font(hb.Face(blob))
        tt = TTFont(path, lazy=True)
        self.cmap = set(tt.getBestCmap())
        glyph_order = tt.getGlyphOrder()
        classes = tt["GDEF"].table.GlyphClassDef.classDefs if "GDEF" in tt else {}
        self.mark_gids = {gid for gid, g in enumerate(glyph_order) if classes.get(g) == MARK_CLASS}
        self.dotted_gid = self.hb_font.get_nominal_glyph(DOTTED_CIRCLE)
        self.axes = [a.axisTag for a in tt["fvar"].axes] if "fvar" in tt else []
        self.features = sorted(
            {fr.FeatureTag for fr in tt["GSUB"].table.FeatureList.FeatureRecord}
            | {fr.FeatureTag for fr in tt["GPOS"].table.FeatureList.FeatureRecord}
        )

    def shape(self, text: str, features: dict | None = None) -> tuple[list, list]:
        buf = hb.Buffer()
        buf.add_str(text)
        buf.guess_segment_properties()
        hb.shape(self.hb_font, buf, features or {})
        return buf.glyph_infos, buf.glyph_positions


def fonts() -> dict[str, Font]:
    found = {}
    for rid, zip_id in OFFICIAL_ZIP.items():
        found[f"KFGQPC {rid}"] = Font(
            f"KFGQPC {rid}", only(sorted(extracted(zip_id).glob("*/*.ttf")))
        )
    src = verified_sources()
    found["KFGQPC bazzi"] = Font("KFGQPC bazzi (mirror v7)", src["kfgqpc-mirror-bazzi-font"])
    found["DK madina"] = Font("DigitalKhatt madina", src["digitalkhatt-madina-font"])
    found["DK oldmadina"] = Font("DigitalKhatt oldmadina", src["digitalkhatt-oldmadina-font"])
    return found


def texts() -> dict[str, list[str]]:
    found = {}
    for rid in RIWAYAT:
        rows = (
            load_json(verified_sources()["kfgqpc-mirror-bazzi-data"])
            if rid == "bazzi"
            else kfgqpc_official(rid)
        )
        found[rid] = [r["aya_text"] for r in rows]  # type: ignore[index]
    for sid, name in (
        ("digitalkhatt-text-madina", "DK madina text"),
        ("digitalkhatt-text-oldmadina", "DK oldmadina text"),
    ):
        found[name] = [line for page in dk_text_pages(sid) for line in page]
    return found


def cp(c: str) -> str:
    return f"U+{ord(c):04X}"


def audit(font: Font, units: list[str]) -> dict:
    notdef: Counter = Counter()
    dotted: Counter = Counter()
    unattached: Counter = Counter()
    missing = Counter(c for u in units for c in u if ord(c) not in font.cmap and not c.isspace())
    for unit in units:
        infos, pos = font.shape(unit)
        _, pos_off = font.shape(unit, NO_MARK_FEATURES)
        # HarfBuzz clusters are UTF-32 indices for Python strings.
        for i, (info, p) in enumerate(zip(infos, pos, strict=True)):
            here = unit[info.cluster : info.cluster + 1] or "?"
            if info.codepoint == 0:
                notdef[cp(here)] += 1
            elif font.dotted_gid and info.codepoint == font.dotted_gid and here != "\u25cc":
                dotted[cp(here)] += 1
            elif info.codepoint in font.mark_gids:
                q = pos_off[i]
                if (p.x_offset, p.y_offset) == (q.x_offset, q.y_offset):
                    unattached[cp(here)] += 1
    return {"missing": missing, "notdef": notdef, "dotted": dotted, "unattached": unattached}


def summarize(counter: Counter) -> str:
    if not counter:
        return "0"
    top = ", ".join(f"{k}x{v}" for k, v in counter.most_common(TOP))
    return f"**{sum(counter.values())}** ({top}{', \u2026' if len(counter) > TOP else ''})"


def inventory_section(all_texts: dict[str, list[str]]) -> None:
    out("## 1. Codepoints used by each text (excluding base letters U+0621-U+064A)")
    out()
    counts = {name: Counter(c for u in units for c in u) for name, units in all_texts.items()}
    union = sorted(set().union(*counts.values()))
    names = list(counts)
    out("| Codepoint | Cat | Name | " + " | ".join(names) + " |")
    out("|---|---|---|" + "---|" * len(names))
    fc_block = 0
    for c in union:
        o = ord(c)
        if 0x0621 <= o <= 0x064A and unicodedata.category(c) == "Lo":
            continue
        if 0xFC00 <= o <= 0xFDFF:
            fc_block += 1
            continue
        cells = " | ".join(str(counts[n][c]) for n in names)
        name = unicodedata.name(c, "?").title()
        out(f"| {cp(c)} | {unicodedata.category(c)} | {name} | {cells} |")
    out()
    out(
        f"Plus {fc_block} codepoints in U+FC00-U+FDFF, which KFGQPC v2 uses as ayah-number "
        "signs (U+FC00 + n - 1). Per text: "
        + ", ".join(f"{n} {sum(1 for c in counts[n] if 0xFC00 <= ord(c) <= 0xFDFF)}" for n in names)
        + "."
    )
    out()


def main() -> int:
    all_texts = texts()
    all_fonts = fonts()
    out("# Font coverage and shaping audit (spike S2)")
    out()
    out(
        f"Generated by `data/spikes/s2_fonts.py` from pinned sources; HarfBuzz "
        f"{hb.__version__} (uharfbuzz). Each KFGQPC text is shaped per ayah exactly as "
        "stored; DigitalKhatt texts per line. No text is altered."
    )
    out()
    inventory_section(all_texts)
    out("## 2. Fonts")
    out()
    out("| Font | File | Glyph classes: marks | Variable axes | OpenType features |")
    out("|---|---|---|---|---|")
    for f in all_fonts.values():
        out(
            f"| {f.name} | `{f.path.name}` | {len(f.mark_gids)} | {', '.join(f.axes) or '\u2014'} "
            f"| {' '.join(f.features)} |"
        )
    out()
    pairs = [(rid, f"KFGQPC {rid}") for rid in RIWAYAT]
    pairs += [
        (rid, f)
        for rid in RIWAYAT
        for f in ("KFGQPC hafs", "DK madina", "DK oldmadina")
        if f != f"KFGQPC {rid}"
    ]
    pairs += [("DK madina text", "DK madina"), ("DK oldmadina text", "DK oldmadina")]
    out("## 3. Shaping audit")
    out()
    out("| Text | Font | Missing from cmap | .notdef | Dotted circles | Unattached marks |")
    out("|---|---|---|---|---|---|")
    for tname, fname in pairs:
        r = audit(all_fonts[fname], all_texts[tname])
        out(
            f"| {tname} | {all_fonts[fname].name} | {summarize(r['missing'])} "
            f"| {summarize(r['notdef'])} | {summarize(r['dotted'])} "
            f"| {summarize(r['unattached'])} |"
        )
        sys.stdout.flush()
    out()
    encoding_section()
    letterform_section(all_fonts)
    return 0


def clusters(word: str) -> list[tuple[str, str]]:
    """Split a word into (base letter, everything attached until the next base letter).

    Base letters are those kept by s1_editions.skeleton(); the base is kept as written so
    that encoding differences of the base itself (e.g. U+0623 vs U+0627+U+0654) show up.
    """
    parts: list[tuple[str, str]] = []
    for c in word:
        if skeleton(c):
            parts.append((c, ""))
        elif parts:
            parts[-1] = (parts[-1][0], parts[-1][1] + c)
        else:
            parts.append(("", c))
    return parts


def fmt(cluster: tuple[str, str]) -> str:
    return " ".join(f"{ord(c):04X}" for c in cluster[0] + cluster[1]) or "-"


def encoding_section() -> None:
    out("## 4. Encoding conventions: KFGQPC Hafs v2.0 vs DigitalKhatt madina text")
    out()
    out(
        "S1 showed both texts have the same letters. Here every word with the same skeleton "
        "is split into clusters (base letter + attached codepoints) and the clusters are "
        "compared one to one. A KFGQPC cluster that always corresponds to the same "
        "DigitalKhatt cluster can be converted by a fixed table; one with several "
        "counterparts depends on context."
    )
    out()
    v2 = {(int(r["sura_no"]), int(r["aya_no"])): kfgqpc_words(r) for r in kfgqpc_official("hafs")}
    counts = [0] * 114
    for s, a in v2:
        counts[s - 1] = max(counts[s - 1], a)
    dk = dk_ayahs("digitalkhatt-text-madina", qul_layout("qul-layout-10-v2-1421h"), counts)
    words = same_words = total_clusters = same_clusters = 0
    pairs: Counter = Counter()
    for key, mine in v2.items():
        theirs = dk.get(key, [])
        if len(mine) != len(theirs):
            continue
        for a, b in zip(mine, theirs, strict=True):
            if skeleton(a) != skeleton(b):
                continue
            words += 1
            same_words += a == b
            ca, cb = clusters(a), clusters(b)
            if len(ca) != len(cb):
                continue
            for x, y in zip(ca, cb, strict=True):
                total_clusters += 1
                if x == y:
                    same_clusters += 1
                else:
                    pairs[(fmt(x), fmt(y))] += 1
    out(f"Aligned words: {words}; identical codepoint for codepoint: {same_words}.")
    out(f"Aligned clusters: {total_clusters}; identical: {same_clusters}.")
    out()
    by_source: dict[str, Counter] = {}
    for (x, y), n in pairs.items():
        by_source.setdefault(x, Counter())[y] += n
    fixed = sum(1 for c in by_source.values() if len(c) == 1)
    out(
        f"Distinct differing KFGQPC cluster forms: {len(by_source)}; with exactly one "
        f"DigitalKhatt counterpart: {fixed}; with several: {len(by_source) - fixed}."
    )
    out()
    out("Most frequent differing forms (codepoints in hex, base letter first):")
    out()
    out("| KFGQPC cluster | Occurrences | DigitalKhatt counterparts (count) |")
    out("|---|---|---|")
    ranked = sorted(by_source.items(), key=lambda kv: -sum(kv[1].values()))
    for x, ys in ranked[:40]:
        alts = "; ".join(f"`{y}` ({n})" for y, n in ys.most_common(4))
        more = f"; +{len(ys) - 4} more" if len(ys) > 4 else ""
        out(f"| `{x}` | {sum(ys.values())} | {alts}{more} |")
    out()


IMG_DIR = Path(__file__).resolve().parents[2] / "docs" / "spikes" / "img"


def render_svg(font: Font, text: str, out_path: Path) -> None:
    import subprocess  # noqa: PLC0415 - only needed for this optional rendering step

    out_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "hb-view",
            "--font-size=64",
            "--margin=16",
            "--output-format=svg",
            f"--output-file={out_path}",
            str(font.path),
            text,
        ],
        check=True,
    )


def letterform_section(all_fonts: dict[str, Font]) -> None:
    out("## 5. Letterforms: Warsh and Qalun text in their own fonts vs the Hafs font")
    out()
    warsh = kfgqpc_official("warsh")
    # First Warsh ayah (in source order) that has both feh and qaf and at most 6 words.
    sample = next(
        r
        for r in warsh
        if "\u0641" in r["aya_text"]
        and "\u0642" in r["aya_text"]
        and len(r["aya_text"].split(" ")) <= 6
    )
    text = sample["aya_text"]
    out(
        f"Sample: Warsh {sample['sura_no']}:{sample['aya_no']} as stored. Rendered with "
        "`hb-view` (no justification). Compare the dots of feh/qaf and final yeh."
    )
    out()
    for key, label in (
        ("KFGQPC warsh", "warsh-font"),
        ("KFGQPC qalun", "qalun-font"),
        ("KFGQPC hafs", "hafs-font"),
    ):
        name = f"letterforms-warsh-{sample['sura_no']}-{sample['aya_no']}-{label}.svg"
        render_svg(all_fonts[key], text, IMG_DIR / name)
        out(f"{all_fonts[key].name}:")
        out()
        out(f"![{label}](spikes/img/{name})")
        out()


if __name__ == "__main__":
    sys.exit(main())
