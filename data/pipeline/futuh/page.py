"""Page structure: table rows (number / reading / explanation cells), footnotes, body text.

Each piece of text is a list of `Token`s in logical order. Quranic text stays at glyph level
(base glyph + attached marks + colour) so later stages can align it letter by letter.
"""

from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass, field

import pymupdf

from .fontmap import FontMap, decode, shape_gids
from .glyphs import Glyph, InkBounds, attach_marks, glyph_text, is_arabic, order_line, page_glyphs

# ---------------------------------------------------------------------------------------------
# Text repair for non-Quran fonts


def repair(gs: list[Glyph], fontmaps: dict[str, FontMap]) -> None:
    """Fix ToUnicode defects we can prove from the font file.

    - Sakkal draws seen/sheen/sad/dad before yeh as a main glyph plus a connector piece (GSUB
      one-to-many). Word put the letter's Unicode on the connector and U+FFFD on the main glyph.
      Connectors get no text; a U+FFFD glyph takes its letter from the font.
    - Presentation-form code points (U+FB50-U+FEFF) are folded to the plain letters."""
    for g in gs:
        fm = fontmaps.get(g.font)
        if fm is not None and g.gid in fm.connectors:
            g.text = ""
        elif "\ufffd" in g.text:
            g.text = (
                g.text.replace("\ufffd", "") or (fm.text(g.gid) if fm is not None else "") or ""
            )
        if any(0xFB50 <= ord(c) <= 0xFEFF for c in g.text):
            g.text = unicodedata.normalize("NFKC", g.text)


# ---------------------------------------------------------------------------------------------
# Tokens


@dataclass
class Token:
    kind: str  # "text" | "quran" | "ayah" | "fn" | "space"
    text: str = ""
    glyphs: list[Glyph] = field(default_factory=list)  # for quran tokens: bases in logical order
    num: int | None = None  # ayah number / footnote number

    def colors(self) -> list[str]:
        return [g.color for g in self.glyphs]


TASHIL_DOT = "\u2022"  # bullet drawn inside Quranic words (hamza place), kept as is
_FN_RE = re.compile(r"^\((\d+)\)$")


def tokenize_line(bases: list[Glyph]) -> list[Token]:
    """One visual line -> logical tokens.

    Quran-font glyphs much smaller than the line's Quranic text are annotations written above
    the text (e.g. qat' / wasl / sakt at a surah junction); they become "label" tokens."""
    out: list[Token] = []
    logical = order_line(bases)
    # Quranic text is set at 11-14 pt; annotation labels at ~7 pt (and may sit on their own line)
    qsize = 12.0
    i = 0
    while i < len(logical):
        g = logical[i]
        if g.quran and not g.is_space and qsize and g.size < 0.8 * qsize:
            j = i
            while j < len(logical) and logical[j].quran and logical[j].size < 0.8 * qsize:
                j += 1
            ws = logical[i:j]
            out.append(Token("label", "".join(glyph_text(x) for x in ws), ws))
            i = j
            continue
        if g.quran and g.quran_ayah_number():
            # consecutive digits form one ayah number (the font draws the ornament per digit run)
            j = i
            digits = ""
            while j < len(logical) and logical[j].quran and logical[j].quran_ayah_number():
                digits += logical[j].text
                j += 1
            out.append(Token("ayah", digits, logical[i:j], int(digits)))
            i = j
            continue
        if (g.quran and not g.is_space and is_arabic(g.text)) or (
            g.text == TASHIL_DOT and i + 1 < len(logical) and logical[i + 1].quran
        ):
            j = i
            # the book marks a hamza's place with a bullet set inside the word (e.g. جٜآ•نَا)
            while j < len(logical) and (
                (
                    logical[j].quran
                    and not logical[j].is_space
                    and not logical[j].quran_ayah_number()
                    and (is_arabic(logical[j].text) or logical[j].text in ("", "\u200d", "\u200c"))
                )
                or logical[j].text == TASHIL_DOT
            ):
                j += 1
            ws = logical[i:j]
            prev = "^"
            for x in ws:
                x.prev_letter = prev
                lt = "".join(
                    c
                    for c in x.text
                    if not unicodedata.category(c).startswith("M") and c != "\u0640"
                )
                prev = lt or prev
            txt = "".join(glyph_text(x) for x in ws)
            # a pause sign set apart by a space belongs to the word before it
            if txt and all(unicodedata.category(c) == "Mn" for c in txt):
                prev = next((t for t in reversed(out) if t.kind != "space"), None)
                if prev is not None and prev.kind == "quran":
                    while out and out[-1].kind == "space":
                        out.pop()
                    prev.glyphs.extend(ws)
                    prev.text += txt
                    i = j
                    continue
            out.append(Token("quran", txt, ws))
            i = j
            continue
        if g.is_space:
            if not out or out[-1].kind != "space":
                out.append(Token("space", " "))
            i += 1
            continue
        # footnote reference: small "(n)" in a non-Quran font
        if g.text == "(" and g.size < 9:
            j = i + 1
            while j < len(logical) and logical[j].text.isdigit():
                j += 1
            if j < len(logical) and logical[j].text == ")" and j > i + 1:
                n = "".join(x.text for x in logical[i + 1 : j])
                out.append(Token("fn", f"({n})", logical[i : j + 1], int(n)))
                i = j + 1
                continue
        t = glyph_text(g)
        if (
            out
            and out[-1].kind == "text"
            and out[-1].glyphs
            and out[-1].glyphs[-1].color == g.color
        ):
            out[-1].text += t
            out[-1].glyphs.append(g)
        else:
            out.append(Token("text", t, [g]))
        i += 1
    return out


def _merge_pause_signs(toks: list[Token]) -> list[Token]:
    """A Quran token that is only marks (a pause sign that wrapped or was spaced off) joins the
    previous Quran word of the row."""
    out: list[Token] = []
    for t in toks:
        if t.kind == "quran" and t.text and all(unicodedata.category(c) == "Mn" for c in t.text):
            prev = next((x for x in reversed(out) if x.kind != "space"), None)
            if prev is not None and prev.kind == "quran":
                while out and out[-1].kind == "space":
                    out.pop()
                prev.glyphs.extend(t.glyphs)
                prev.text += t.text
                continue
        out.append(t)
    return out


def tokens_text(tokens: list[Token]) -> str:
    """Plain text; annotation labels are shown as \u27e8label\u27e9."""
    return "".join(f"\u27e8{t.text}\u27e9" if t.kind == "label" else t.text for t in tokens)


# ---------------------------------------------------------------------------------------------
# Lines


def build_lines(bases: list[Glyph], tol: float = 3.0, gap: float = 18.0) -> list[list[Glyph]]:
    """Cluster base glyphs into visual lines by baseline, split at wide horizontal gaps.

    Returns lines top-to-bottom; within a line, glyphs in visual order."""
    rows: list[list[Glyph]] = []
    for b in sorted(bases, key=lambda g: (g.y, g.x0)):
        for r in rows:
            if abs(r[0].y - b.y) < tol:
                r.append(b)
                break
        else:
            rows.append([b])
    out = []
    for r in sorted(rows, key=lambda r: r[0].y):
        r.sort(key=lambda g: g.cx)
        segs = [[r[0]]]
        for g in r[1:]:
            if g.x0 - segs[-1][-1].x1 > gap and not g.is_space:
                segs.append([g])
            else:
                segs[-1].append(g)
        out.extend(reversed(segs))  # right segment first
    return [s for s in out if any(not g.is_space for g in s)]


# ---------------------------------------------------------------------------------------------
# Page model


@dataclass
class Row:
    y0: float
    y1: float
    group_start: bool  # a thick rule above this row starts a new segment group
    number: str = ""
    reading: list[Token] = field(default_factory=list)
    explanation: str = ""


@dataclass
class PageModel:
    pdf_page: int  # 1-based
    printed_page: str | None
    rows: list[Row]
    body_before: list[list[Token]]  # text lines above the table
    body_after: list[list[Token]]  # text lines between table and footnote rule
    footnotes: list[list[Token]]  # lines below the footnote rule
    orphan_marks: int
    roundtrip: dict  # counts per verdict for prose lines


def _cells(
    page: pymupdf.Page,
) -> tuple[list[tuple[float, float, bool]], tuple[float | None, float] | None]:
    """Find table rows from the filled cell rectangles of the number column.

    Returns (rows as (y0, y1, group_start), x bounds (expl_x1, num_x0))."""
    num_rects, expl_rects, thick = [], [], []
    for d in page.get_drawings():
        for it in d["items"]:
            if it[0] == "re" and d.get("fill") is not None:
                r = it[1]
                if 15 < r.height < 200 and 18 < r.width < 30 and r.x0 > 380:
                    num_rects.append(r)
                elif 15 < r.height < 200 and 90 < r.width < 140 and r.x1 < 200:
                    expl_rects.append(r)
            elif it[0] == "l" and (d.get("width") or 0) >= 3.5:
                a, b = it[1], it[2]
                if abs(a.y - b.y) < 0.5 and abs(a.x - b.x) > 15:
                    thick.append(a.y)
    num_rects.sort(key=lambda r: r.y0)
    uniq = []
    for r in num_rects:
        if not uniq or abs(uniq[-1].y0 - r.y0) > 2:
            uniq.append(r)
    rows = []
    for r in uniq:
        gs = any(abs(t - r.y0) < 4 for t in thick)
        rows.append((r.y0, r.y1, gs))
    if not uniq:
        return [], None
    num_x0 = min(r.x0 for r in uniq)
    expl_x1 = max((r.x1 for r in expl_rects), default=None)
    return rows, (expl_x1, num_x0)


def _footnote_rule_y(lines: list[list[Glyph]]) -> float | None:
    for line in lines:
        t = "".join(g.text for g in line).strip()
        if len(t) >= 15 and set(t) <= {"_"}:
            return line[0].y
    return None


def parse_page(
    doc: pymupdf.Document, pno: int, ink: InkBounds, fontmaps: dict[str, FontMap]
) -> PageModel:
    page = doc[pno]
    gs = page_glyphs(page, ink)
    repair(gs, fontmaps)
    bases = [g for g in gs if not g.is_mark]
    marks = [g for g in gs if g.is_mark]
    orphans = attach_marks(bases, marks)

    # printed page number: bold Times digits at the bottom
    printed = None
    pn_glyphs = [
        g
        for g in bases
        if g.font.startswith("TimesNewRoman") and g.y > page.rect.height * 0.9 and g.text.isdigit()
    ]
    if pn_glyphs:
        printed = "".join(g.text for g in sorted(pn_glyphs, key=lambda g: g.cx))
    bases = [g for g in bases if g not in pn_glyphs]
    # running-header page number at the top (later volumes): drop
    bases = [
        g
        for g in bases
        if not (g.font.startswith("KFGQPCUthmanTahaNaskh-Bo") and g.y < 95 and g.text.isdigit())
    ]

    rows_geom, xb = _cells(page)
    rows: list[Row] = []
    in_table: set[int] = set()
    rt = {"exact": 0, "equivalent": 0, "corrected": 0, "fail": 0, "unchecked": 0}
    if rows_geom and xb:
        expl_x1, num_x0 = xb
        for y0, y1, gstart in rows_geom:
            row = Row(y0, y1, gstart)
            cell = [g for g in bases if y0 - 1 <= g.y <= y1 + 4]
            for g in cell:
                in_table.add(id(g))
            num = [g for g in cell if g.cx >= num_x0]
            expl = [g for g in cell if expl_x1 is not None and g.cx <= expl_x1]
            read = [g for g in cell if g not in num and g not in expl]
            row.number = "".join(g.text for g in sorted(num, key=lambda g: g.cx)).strip()
            row.explanation = " ".join(
                _line_text(line, fontmaps, rt) for line in build_lines(expl)
            ).strip()
            toks: list[Token] = []
            for line in build_lines(read, gap=60):
                if toks:
                    toks.append(Token("space", " "))
                toks.extend(tokenize_line(line))
            row.reading = _merge_pause_signs(toks)
            rows.append(row)

    # a segment opening right under the column-header row has no thick rule above it
    first = next((r for r in rows if r.number.strip().isdigit()), None)
    if first is not None and first.number.strip() == "1":
        first.group_start = True

    rest = [g for g in bases if id(g) not in in_table]
    lines = build_lines(rest)
    rule_y = _footnote_rule_y(lines)
    table_top = min((r.y0 for r in rows), default=None)
    before, after, foot = [], [], []
    for line in lines:
        t = "".join(g.text for g in line).strip()
        if set(t) <= {"_"}:
            continue
        _check_line(line, fontmaps, rt)
        toks = tokenize_line(line)
        y = line[0].y
        if rule_y is not None and y > rule_y:
            foot.append(toks)
        elif table_top is not None and y < table_top:
            before.append(toks)
        else:
            after.append(toks)
    return PageModel(pno + 1, printed, rows, before, after, foot, len(orphans), rt)


def _line_text(line: list[Glyph], fontmaps: dict[str, FontMap], rt: dict) -> str:
    _check_line(line, fontmaps, rt)
    return tokens_text(tokenize_line(line))


# ---------------------------------------------------------------------------------------------
# Prose round-trip


_DIGITS = str.maketrans(
    "\u0660\u0661\u0662\u0663\u0664\u0665\u0666\u0667\u0668\u0669", "0123456789"
)


def _verdict(fm: FontMap, run: list[Glyph]) -> tuple[str, list, str]:
    """Round-trip one single-font run. Returns (verdict, diffs) where each diff is
    (our gids, pdf gids, pdf base glyph objects)."""
    import difflib

    text = "".join(glyph_text(g) for g in order_line(run))
    pdf_b = [g for g in run if g.gid not in fm.spaces and g.gid not in fm.marks]
    vis_m = [m.gid for g in run for m in g.marks if m.gid in fm.marks]
    shaped = shape_gids(fm.path, text)
    ours_b = [g for g in shaped if g not in fm.spaces and g not in fm.marks]
    ours_m = sorted(g for g in shaped if g in fm.marks)
    sm = difflib.SequenceMatcher(a=ours_b, b=[g.gid for g in pdf_b], autojunk=False)
    diffs = [
        (ours_b[i1:i2], [g.gid for g in pdf_b[j1:j2]], pdf_b[j1:j2])
        for op, i1, i2, j1, j2 in sm.get_opcodes()
        if op != "equal"
    ]
    marks_ok = ours_m == sorted(vis_m) or sorted(decode(fm, ours_m)) == sorted(decode(fm, vis_m))
    if not diffs and ours_m == sorted(vis_m):
        return "exact", diffs, text
    ok = marks_ok
    for o, p, _ in diffs:
        a, b = decode(fm, o).translate(_DIGITS), decode(fm, p).translate(_DIGITS)
        if a == b:
            continue
        if "?" in b and len(a) == len(b) and all(y in (x, "?") for x, y in zip(a, b, strict=False)):
            continue  # glyph newer than our font file: same letter count, rest agrees
        ok = False
    return ("equivalent" if ok else "fail"), diffs, text


def _check_line(line: list[Glyph], fontmaps: dict[str, FontMap], rt: dict) -> None:
    """Round-trip each single-font run of the line through HarfBuzz and tally the verdicts.

    On failure, the text of the PDF glyphs in each mismatching stretch is replaced by what the
    font file says those glyphs are, and the run is checked again ("corrected")."""
    runs: list[list[Glyph]] = []
    for g in sorted(line, key=lambda g: g.cx):
        if runs and runs[-1][-1].font == g.font:
            runs[-1].append(g)
        else:
            runs.append([g])
    for run in runs:
        fm = fontmaps.get(run[0].font)
        if all(g.is_space or g.text == "" for g in run):
            continue
        if fm is None:
            rt["unchecked"] += 1
            continue
        verdict, diffs, text = _verdict(fm, run)
        if verdict == "fail":
            before = text
            for _, _p, objs in diffs:
                for g in objs:
                    t = fm.text(g.gid)
                    if t is not None:
                        g.text = unicodedata.normalize("NFKC", t).replace("\u0640", "")
            verdict2, diffs2, text2 = _verdict(fm, run)
            if verdict2 != "fail":
                verdict = "corrected"
                rt.setdefault("corrections", []).append({"before": before, "after": text2})
            else:
                diffs, text = diffs2, text2
        rt[verdict] = rt.get(verdict, 0) + 1
        if verdict == "fail":
            rt.setdefault("examples", []).append(
                {"text": text, "diffs": [(o, p, decode(fm, o), decode(fm, p)) for o, p, _ in diffs]}
            )
