"""Glyph-level reading of a PDF page and logical (RTL) reordering.

Why glyph level: the PDF's ToUnicode entries are mostly correct *per glyph*, in logical order
(e.g. one ligature glyph carries ha-meem-noon with their marks). Ordinary extractors reverse
each RTL run character by character, which scrambles every multi-character glyph (lam-alif
ligatures come out alif-lam).
Here a glyph is the unit: we reorder glyphs, never the characters inside one glyph.
"""

from __future__ import annotations

import functools
import sys
import unicodedata
from dataclasses import dataclass, field

import pymupdf

QURAN_FONTS = ("KFGQPCUthmanicScriptHAFS", "KFGQPCHAFSUthmanicScript")


def font_base(name: str) -> str:
    return name.split("+", 1)[-1]


def is_quran_font(name: str) -> bool:
    return font_base(name).startswith(QURAN_FONTS)


def hex_color(rgb: int | tuple[float, ...]) -> str:
    if isinstance(rgb, int):
        return f"#{rgb:06x}"
    r, g, b = (max(0, min(255, round(v * 255))) for v in rgb[:3])
    return f"#{r:02x}{g:02x}{b:02x}"


def _is_mark_text(t: str) -> bool:
    return bool(t) and all(unicodedata.category(c) in ("Mn", "Me") for c in t)


@dataclass
class Glyph:
    gid: int
    text: str  # this glyph's ToUnicode string (logical order inside the glyph)
    font: str
    color: str
    size: float
    x0: float
    x1: float
    y: float  # baseline (origin y)
    seq: int  # content-stream order on the page
    marks: list[Glyph] = field(default_factory=list)  # attached combining marks (bases only)
    ink_x: tuple[float, float] | None = None  # drawn extent of a zero-advance glyph
    prev_letter: str = "^"  # ToUnicode letters of the preceding glyph in the word ("^" = first)

    @property
    def mx(self) -> float:
        """Horizontal centre where the glyph is actually drawn."""
        if self.ink_x is not None:
            return (self.ink_x[0] + self.ink_x[1]) / 2
        return self.cx

    @property
    def is_mark(self) -> bool:
        # A mark has zero advance; most also have mark-only text. Zero-width bases with letter
        # text exist in this PDF only as ligature continuation pieces, which we never create.
        # Zero-advance glyphs are marks; so is any glyph whose text is only combining marks,
        # even when the font gives it an advance (e.g. the imala dot below, U+065C).
        if _is_mark_text(self.text):
            return True
        return (self.x1 - self.x0) < 0.05 and self.text == ""

    @property
    def is_space(self) -> bool:
        return self.text != "" and self.text.strip() == "" and not self.is_mark

    @property
    def cx(self) -> float:
        return (self.x0 + self.x1) / 2

    @property
    def quran(self) -> bool:
        return is_quran_font(self.font)


class InkBounds:
    """Per-font glyph ink extents (xMin, xMax in em units), merged from every embedded subset.

    Subsets of one font keep the original glyph ids, so bounds for a gid agree across subsets."""

    def __init__(self, doc: pymupdf.Document) -> None:
        self.doc = doc
        self.seen_xrefs: set[int] = set()
        self.bounds: dict[str, dict[int, tuple[float, float]]] = {}

    def load_page(self, page: pymupdf.Page) -> None:
        import io

        from fontTools.ttLib import TTFont

        for f in page.get_fonts(full=True):
            xref, name = f[0], font_base(f[3])
            if xref in self.seen_xrefs:
                continue
            self.seen_xrefs.add(xref)
            try:
                buf = self.doc.extract_font(xref)[3]
                tt = TTFont(io.BytesIO(buf), lazy=True)
                glyf = tt["glyf"]
                upem = tt["head"].unitsPerEm
            except (RuntimeError, ValueError, KeyError) as e:  # font not readable
                print(f"futuh: font xref {xref} skipped: {e}", file=sys.stderr)
                continue
            d = self.bounds.setdefault(name, {})
            for gid, gname in enumerate(tt.getGlyphOrder()):
                if gid in d:
                    continue
                gl = glyf[gname]
                if gl.numberOfContours == 0:
                    continue
                gl.recalcBounds(glyf)
                d[gid] = (gl.xMin / upem, gl.xMax / upem)

    def ink(self, font: str, gid: int) -> tuple[float, float] | None:
        d = self.bounds.get(font)
        if d is None:
            # the embedded name can be a truncated/suffixed form ("...Script-Regula")
            for k, v in self.bounds.items():
                if k.startswith(font) or font.startswith(k):
                    d = self.bounds[font] = v
                    break
        return (d or {}).get(gid)


def page_glyphs(page: pymupdf.Page, ink: InkBounds | None = None) -> list[Glyph]:
    """All glyphs on the page. Continuation chars (gid -1) are folded into their glyph.

    With `ink`, zero-advance glyphs (marks) get their drawn ink extent as x0..x1 (ink_x),
    which is where the mark really sits; the advance box of a mark is just the pen position."""
    if ink is not None:
        ink.load_page(page)
    out: list[Glyph] = []
    seq = 0
    for span in page.get_texttrace():
        font = font_base(span["font"])
        color = hex_color(span["color"])
        size = span["size"]
        cur: Glyph | None = None
        for u, gid, origin, bbox in span["chars"]:
            ch = chr(u) if u > 0 else ""
            if gid == -1 and cur is not None:
                cur.text += ch
                continue
            cur = Glyph(gid, ch, font, color, size, bbox[0], bbox[2], origin[1], seq)
            if ink is not None and (bbox[2] - bbox[0]) < 0.05:
                b = ink.ink(font, gid)
                if b is not None:
                    cur.ink_x = (origin[0] + b[0] * size, origin[0] + b[1] * size)
            seq += 1
            out.append(cur)
    return out


# ---------------------------------------------------------------------------------------------
# Logical ordering

_LTR_CATS = ("L",)  # strong LTR bidi class


def _bidi(g: Glyph) -> str:
    t = g.text
    if not t:
        return "ON"
    cls = unicodedata.bidirectional(t[0])
    return cls


def _is_ltr_like(g: Glyph) -> bool:
    return _bidi(g) in ("L", "EN", "AN") and not g.quran_ayah_number()


def _quran_ayah_number(self: Glyph) -> bool:
    # In the KFGQPC Uthmanic font an ASCII digit draws an ayah-end ornament with that number.
    # (the newer KFGQPC font draws a whole number like "٧٦" as one glyph)
    return self.quran and self.text != "" and self.text.isdigit()


Glyph.quran_ayah_number = _quran_ayah_number  # type: ignore[attr-defined]


_BELOW_MARKS = set("\u0650\u064d\u0655\u065c\u06e3\u06ea\u06ed\u0656\u08f2\u0325\u0323")


def attach_marks(bases: list[Glyph], marks: list[Glyph]) -> list[Glyph]:
    """Attach each mark to the base whose advance box contains it (nearest otherwise).

    Returns marks that could not be attached (no base near enough)."""
    orphans = []
    by_seq = {g.seq: g for g in bases}
    by_seq.update({g.seq: g for g in marks})
    for m in marks:
        # Word writes each mark immediately before its base in the content stream (visual LTR
        # order of a reversed RTL run). Trust that when the geometry agrees; it is exact even
        # where a mark is drawn over the neighbouring letter (narrow initial yeh/noon teeth).
        nxt = None
        for s in range(m.seq + 1, m.seq + 6):
            g = by_seq.get(s)
            if g is None:
                break
            if g.is_mark:
                continue
            nxt = g
            break
        if (
            nxt is not None
            and not nxt.is_space
            and nxt.quran == m.quran
            and -2.0
            < nxt.y - m.y
            < max(nxt.size, m.size) * 1.0  # marks sit on or above their base line
            and nxt.x0 - 1.5 <= m.mx <= nxt.x1 + 1.5
        ):
            nxt.marks.append(m)
            continue
        best, bestd = None, 1e9
        above_only = not any(c in _BELOW_MARKS for c in m.text)
        for b in bases:
            if b.is_space:
                continue
            if above_only and m.y > b.y + 1.5:
                continue  # a mark drawn above a letter never sits below that letter's baseline
            dy = abs(b.y - m.y)
            if dy > max(b.size, m.size) * 1.2:
                continue
            mx = m.mx
            fam = 0.0 if b.quran == m.quran else 2.0
            if b.x0 - 0.3 <= mx <= b.x1 + 0.3:
                d = dy / b.size + 0.3 * abs(mx - b.cx) / max(b.x1 - b.x0, 0.5) + fam
            else:
                d = 1.5 + min(abs(mx - b.x0), abs(mx - b.x1)) + dy / b.size + fam
            if d < bestd:
                best, bestd = b, d
        if best is None or bestd > 6:
            orphans.append(m)
        else:
            best.marks.append(m)
    return orphans


def order_line(bases: list[Glyph]) -> list[Glyph]:
    """Visual -> logical order for one line of base glyphs (marks already attached).

    Paragraph direction is RTL. Sort by x descending, then restore runs of LTR text
    (digits, Latin, and neutrals between them) to ascending x, as the bidi algorithm would."""
    rtl = sorted(bases, key=lambda g: (-g.cx, g.seq))
    out: list[Glyph] = []
    i = 0
    n = len(rtl)
    while i < n:
        if _is_ltr_like(rtl[i]):
            j = i + 1
            # extend over LTR glyphs and neutrals that are followed by more LTR (e.g. "220-120")
            while j < n:
                if _is_ltr_like(rtl[j]):
                    j += 1
                    continue
                if (
                    _bidi(rtl[j]) in ("CS", "ES", "ET", "ON")
                    and not rtl[j].is_space
                    and j + 1 < n
                    and _is_ltr_like(rtl[j + 1])
                ):
                    j += 2
                    continue
                break
            out.extend(reversed(rtl[i:j]))
            i = j
        else:
            out.append(rtl[i])
            i += 1
    return out


def is_arabic(t: str) -> bool:
    """True if the text has an Arabic letter or mark (not just punctuation/digits)."""
    return any(
        (
            0x0600 <= ord(c) <= 0x06FF
            and not c.isdigit()
            and c not in "\u060c\u061b\u061f\u066a\u066b\u066c\u06d4"
        )
        or 0x08A0 <= ord(c) <= 0x08FF
        for c in t
    )


@functools.cache
def proven_glyphs() -> set[str]:
    """Quran-font glyphs whose decoding was shown consistent with Hafs by futuh/learn.py."""
    import json

    from .paths import BOOK

    f = BOOK / "glyphfix.json"
    return set(json.loads(f.read_text()).get("proven", [])) if f.exists() else set()


@functools.cache
def glyph_fixes() -> dict:
    """Learned per-glyph corrections for the Quran fonts (futuh/learn.py -> glyphfix.json)."""
    import json

    from .paths import BOOK

    f = BOOK / "glyphfix.json"
    return json.loads(f.read_text())["glyphs"] if f.exists() else {}


def glyph_text(g: Glyph) -> str:
    """Text of a base glyph plus its marks.

    For a Quran-font glyph with a learned fix, its letters are replaced by the learned ones
    and each mark goes to the component whose learned position is nearest."""
    ms = sorted(g.marks, key=lambda m: m.seq)
    own = "".join(c for c in g.text if unicodedata.category(c) != "Mn" and c != "\u0640")
    fx = None
    if g.quran and (is_arabic(own) or g.text == ""):
        fx = glyph_fixes().get(f"{g.font}:{g.gid}:{own}|{g.prev_letter}") or glyph_fixes().get(
            f"{g.font}:{g.gid}:{own}"
        )
    has_marks_inside = any(unicodedata.category(c) == "Mn" for c in g.text)
    letters = fx["letters"] if fx and not has_marks_inside else own
    if g.quran and ms and len(letters) >= 2 and not has_marks_inside and g.x1 - g.x0 >= 0.5:
        return _distribute(
            g,
            letters,
            ms,
            {int(k): v for k, v in (fx or {}).get("comp_r", {}).items()},
            (fx or {}).get("comp_rc", {}),
        )
    if fx and not has_marks_inside:
        return letters + "".join(m.text for m in ms)
    if not ms:
        return g.text
    return g.text + "".join(m.text for m in ms)


def _distribute(
    g: Glyph, letters: str, ms: list[Glyph], comp: dict[int, float], comp_rc: dict | None = None
) -> str:
    """Put the marks of a multi-letter (ligature) glyph on its letters.

    r = 0 at the glyph's right edge (first letter), 1 at its left edge (last letter).
    0. If a learned position for this mark character exists (glyphfix.json comp_rc), use the
       nearest component that carries that character.
    1. If the marks form exactly one positional cluster per letter, assign clusters in order.
    2. Else, if a learned position per component exists (glyphfix.json), use the nearest.
    3. Else, split the glyph into equal slices."""
    w = g.x1 - g.x0
    n = len(letters)
    if comp_rc or comp:
        # per mark: the learned position for its character when it fits (within 0.15 of the
        # glyph width), else the learned per-component position, else equal slices
        per0: list[list[str]] = [[] for _ in letters]
        for m in sorted(ms, key=lambda m: m.seq):
            r = (g.x1 - m.mx) / w
            cands = {int(k): v for k, v in (comp_rc or {}).get(m.text[:1], {}).items()}
            k = min(cands, key=lambda k: abs(cands[k] - r)) if cands else None
            if k is None or abs(cands[k] - r) > 0.15:
                k = (
                    min(comp, key=lambda k: abs(comp[k] - r))
                    if comp
                    else int(max(0.0, min(0.999, r)) * n)
                )
            per0[min(k, n - 1)].append(m.text)
        return "".join(ch + "".join(p) for ch, p in zip(letters, per0, strict=False))
    rs = [((g.x1 - m.mx) / w, m) for m in ms]
    rs.sort(key=lambda t: t[0])
    clusters_: list[list[tuple[float, Glyph]]] = []
    for r, m in rs:
        if clusters_ and r - clusters_[-1][-1][0] < 0.6 / w:
            clusters_[-1].append((r, m))
        else:
            clusters_.append([(r, m)])
    per: list[list[str]] = [[] for _ in letters]
    if len(clusters_) == n:
        for k, cl in enumerate(clusters_):
            per[k].extend(m.text for _, m in sorted(cl, key=lambda t: t[1].seq))
    else:
        for r, m in sorted(rs, key=lambda t: t[1].seq):
            if comp:
                k = min(comp, key=lambda k: abs(comp[k] - r))
            else:
                k = int(max(0.0, min(0.999, r)) * n)
            per[min(k, n - 1)].append(m.text)
    return "".join(ch + "".join(p) for ch, p in zip(letters, per, strict=False))
