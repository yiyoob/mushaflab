"""Decode glyph ids through a real font file, and round-trip text through HarfBuzz.

`FontMap` inverts cmap + GSUB (single, multiple-to-one ligature, alternate) so every glyph id
reachable by shaping gets the character string that produces it. `roundtrip` shapes our
logical text with the same font and compares glyph ids with the ones the PDF actually drew:
if they agree, the text we recovered provably renders as the printed page.
"""

from __future__ import annotations

from collections import Counter
from collections.abc import Iterable
from functools import lru_cache
from pathlib import Path
from typing import TYPE_CHECKING

from fontTools.ttLib import TTFont

if TYPE_CHECKING:
    import uharfbuzz


class FontMap:
    def __init__(self, path: Path) -> None:
        self.path = Path(path)
        tt = TTFont(str(path))
        order = tt.getGlyphOrder()
        gid = {n: i for i, n in enumerate(order)}
        m: dict[int, str] = {}
        for cp, name in tt.getBestCmap().items():
            g = gid[name]
            # prefer the plain Arabic letter over presentation forms when a glyph has both
            if g not in m or (0xFB50 <= ord(m[g][0]) <= 0xFEFF):
                m[g] = chr(cp)
        subs: list[tuple[str, int, list[int]]] = []  # (kind, out, inputs)
        self.connectors: set[int] = set()  # extra outputs of one-to-many substitutions: no text
        if "GSUB" in tt:
            for lookup in tt["GSUB"].table.LookupList.Lookup:
                for sub in lookup.SubTable:
                    st = sub.ExtSubTable if sub.LookupType == 7 else sub
                    t = st.LookupType
                    if t == 1:
                        for a, b in st.mapping.items():
                            subs.append(("s", gid[b], [gid[a]]))
                    elif t == 2:
                        for a, outs in st.mapping.items():
                            subs.append(("s", gid[outs[0]], [gid[a]]))
                            self.connectors.update(gid[b] for b in outs[1:])
                    elif t == 3:
                        for a, alts in st.alternates.items():
                            for b in alts:
                                subs.append(("s", gid[b], [gid[a]]))
                    elif t == 4:
                        for first, ligs in st.ligatures.items():
                            for lig in ligs:
                                subs.append(
                                    (
                                        "l",
                                        gid[lig.LigGlyph],
                                        [gid[first]] + [gid[c] for c in lig.Component],
                                    )
                                )
        changed = True
        while changed:
            changed = False
            for _, out, ins in subs:
                if out in m:
                    continue
                if all(i in m for i in ins):
                    m[out] = "".join(m[i] for i in ins)
                    changed = True
        for c in self.connectors:
            m[c] = ""
        self.map = m
        self.marks: set[int] = set()
        if "GDEF" in tt and tt["GDEF"].table.GlyphClassDef:
            self.marks = {
                gid[n] for n, c in tt["GDEF"].table.GlyphClassDef.classDefs.items() if c == 3
            }
        self.spaces = {g for g, s in m.items() if s.strip() == ""}

    def text(self, gid: int) -> str | None:
        return self.map.get(gid)


@lru_cache(maxsize=4)
def _hb_font(path: str) -> uharfbuzz.Font:
    import uharfbuzz as hb

    blob = hb.Blob(Path(path).read_bytes())
    face = hb.Face(blob)
    return hb.Font(face)


def shape_gids(path: Path, text: str) -> list[int]:
    """Glyph ids HarfBuzz produces for `text` (RTL Arabic), in visual left-to-right order."""
    import uharfbuzz as hb

    font = _hb_font(str(path))
    buf = hb.Buffer()
    buf.add_str(text)
    buf.direction = "rtl"
    buf.script = "Arab"
    buf.language = "ar"
    hb.shape(font, buf, {})
    return [i.codepoint for i in buf.glyph_infos]  # RTL buffers come out in visual LTR order


def decode(fm: FontMap, gids: Iterable[int]) -> str:
    import unicodedata

    return unicodedata.normalize("NFKC", "".join(fm.text(g) or "?" for g in gids)).replace(
        "\u0640", ""
    )


def base_diffs(
    fm: FontMap, text: str, pdf_gids_visual: list[int]
) -> list[tuple[tuple[int, ...], tuple[int, ...]]]:
    """Non-matching stretches between HarfBuzz's bases and the PDF's bases: (ours, pdf)."""
    import difflib

    ours = [g for g in shape_gids(fm.path, text) if g not in fm.spaces and g not in fm.marks]
    theirs = [g for g in pdf_gids_visual if g not in fm.spaces and g not in fm.marks]
    sm = difflib.SequenceMatcher(a=ours, b=theirs, autojunk=False)
    return [
        (tuple(ours[i1:i2]), tuple(theirs[j1:j2]))
        for op, i1, i2, j1, j2 in sm.get_opcodes()
        if op != "equal"
    ]


def roundtrip(
    path: Path, text: str, pdf_gids_visual: list[int], marks: set[int], spaces: set[int]
) -> tuple[bool, str]:
    """Compare HarfBuzz output for `text` with the glyphs the PDF drew.

    Base glyphs must match in order; marks must match as a multiset (their order in the PDF
    content stream is positional, not logical). Spaces are ignored."""
    ours = [g for g in shape_gids(path, text) if g not in spaces]
    theirs = [g for g in pdf_gids_visual if g not in spaces]
    ob = [g for g in ours if g not in marks]
    tb = [g for g in theirs if g not in marks]
    om = Counter(g for g in ours if g in marks)
    tm = Counter(g for g in theirs if g in marks)
    if ob == tb and om == tm:
        return True, ""
    why = []
    if ob != tb:
        i = next(
            (k for k, (a, b) in enumerate(zip(ob, tb, strict=False)) if a != b),
            min(len(ob), len(tb)),
        )
        why.append(
            f"bases differ at {i}: ours {ob[i : i + 4]} pdf {tb[i : i + 4]} "
            f"(len {len(ob)} vs {len(tb)})"
        )
    if om != tm:
        why.append(f"marks differ: +{dict(om - tm)} -{dict(tm - om)}")
    return False, "; ".join(why)
