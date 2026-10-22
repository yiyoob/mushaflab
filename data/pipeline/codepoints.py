"""The codepoint class table: pipeline/codepoint_classes.tsv (CLAUDE.md conventions).

One row per codepoint used by any of the 8 KFGQPC texts. The class decides clustering;
the other columns are evidence computed from pinned sources and checked by tests:
  * category, name: Unicode data (Python's unicodedata);
  * gdef: the GDEF glyph class each KFGQPC font gives the codepoint's glyph, with counts;
  * used_by: the riwayat whose text uses it.
A class that differs from what the fonts' GDEF suggests must carry a note.

    python -m pipeline.codepoints generate   # write the table from the evidence
    python -m pipeline.codepoints check      # exit 1 if the table is stale or incomplete
    python -m pipeline.codepoints review     # write docs/review/codepoint-classes.md
"""

import sys
import unicodedata
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path

from pipeline import kfgqpc

CLASSES = ("base", "mark", "joiner", "sign", "ayah_number", "digit", "space", "control")
TABLE_PATH = Path(__file__).with_name("codepoint_classes.tsv")
HEADER = ("codepoint", "class", "category", "name", "gdef", "used_by", "note")
GDEF_NAMES = {1: "base", 2: "ligature", 3: "mark", 4: "component"}
AYAH_SIGNS = range(0xFC00, 0xFE00)  # KFGQPC v2 ayah-number signs (G-013)
# U+200D ZERO WIDTH JOINER extends the preceding grapheme cluster (Unicode UAX #29,
# Grapheme_Cluster_Break=ZWJ), so marks after it still belong to that cluster.
JOINERS = {0x200D}


class CodepointError(ValueError):
    pass


@dataclass(frozen=True)
class Row:
    cp: int
    cls: str
    category: str
    name: str
    gdef: str
    used_by: str
    note: str


def proposed_class(cp: int, category: str) -> str:
    """The class implied by the Unicode category (plus KFGQPC's ayah-sign convention)."""
    if cp in AYAH_SIGNS:
        return "ayah_number"
    if cp in JOINERS:
        return "joiner"
    if category in ("Mn", "Me"):
        return "mark"
    if category in ("Lo", "Lm"):
        return "base"
    return {"So": "sign", "Nd": "digit", "Zs": "space", "Cf": "control"}[category]


def expected_gdef(cls: str) -> str:
    return "mark" if cls == "mark" else "base"


def _fmt(cp: int) -> str:
    return f"U+{cp:04X}"


def load_table(path: Path = TABLE_PATH) -> dict[int, Row]:
    lines = path.read_text(encoding="ascii").splitlines()
    if tuple(lines[0].split("\t")) != HEADER:
        raise CodepointError("unexpected header")
    rows: dict[int, Row] = {}
    for n, line in enumerate(lines[1:], start=2):
        fields = line.split("\t")
        if len(fields) != len(HEADER) or not fields[0].startswith("U+"):
            raise CodepointError(f"line {n}: malformed")
        cp = int(fields[0][2:], 16)
        if cp in rows:
            raise CodepointError(f"line {n}: duplicate {_fmt(cp)}")
        if fields[1] not in CLASSES:
            raise CodepointError(f"line {n}: unknown class {fields[1]!r}")
        rows[cp] = Row(cp, *fields[1:])
    return rows


def write_table(rows: dict[int, Row], path: Path = TABLE_PATH) -> None:
    out = ["\t".join(HEADER)]
    for cp in sorted(rows):
        r = rows[cp]
        out.append("\t".join((_fmt(cp), r.cls, r.category, r.name, r.gdef, r.used_by, r.note)))
    path.write_text("\n".join(out) + "\n", encoding="ascii")


def collect_evidence() -> dict[int, tuple[str, str, str, str]]:
    used: dict[int, list[str]] = defaultdict(list)
    gdef: dict[int, Counter] = defaultdict(Counter)
    for riwayah in kfgqpc.RIWAYAT:
        font = kfgqpc.font(riwayah)
        cmap = font.getBestCmap()
        classes = font["GDEF"].table.GlyphClassDef.classDefs
        for cp in sorted({ord(c) for r in kfgqpc.rows(riwayah) for c in r["aya_text"]}):
            used[cp].append(riwayah)
            glyph = cmap.get(cp)
            kind = "nocmap" if glyph is None else GDEF_NAMES.get(classes.get(glyph), "none")
            gdef[cp][kind] += 1
    return {
        cp: (
            unicodedata.category(chr(cp)),
            unicodedata.name(chr(cp), "UNNAMED"),
            ",".join(f"{k}:{v}" for k, v in sorted(gdef[cp].items())),
            ",".join(used[cp]),
        )
        for cp in used
    }


def _note(cls: str, category: str, gdef: str) -> str:
    kinds = {part.split(":")[0] for part in gdef.split(",")}
    if kinds == {expected_gdef(cls)}:
        return ""
    return (
        f"REVIEW: Unicode {category} gives class {cls}; KFGQPC font GDEF says {gdef}. "
        f"Kept as {cls} so clustering follows Unicode."
    )


def generate() -> dict[int, Row]:
    rows = {}
    for cp, (category, name, gdef, used_by) in collect_evidence().items():
        cls = proposed_class(cp, category)
        rows[cp] = Row(cp, cls, category, name, gdef, used_by, _note(cls, category, gdef))
    return rows


REVIEW_PATH = Path(__file__).resolve().parents[2] / "docs" / "review" / "codepoint-classes.md"


def write_review(rows: dict[int, Row]) -> None:
    flagged = [r for r in rows.values() if r.note or r.cls in ("joiner", "control", "digit")]
    out = [
        "# Codepoint classes: rows for owner review",
        "",
        "Generated by `python -m pipeline.codepoints review` from",
        "`data/pipeline/codepoint_classes.tsv`. Classes follow the Unicode category; these rows",
        "are where a KFGQPC font's GDEF class disagrees, or where the class is unusual.",
        f"Unicode data version: {unicodedata.unidata_version}. Total rows: {len(rows)}.",
        "",
        "| Codepoint | Name | Class | Unicode | KFGQPC GDEF (fonts) | Used by | Note |",
        "|---|---|---|---|---|---|---|",
    ]
    for r in sorted(flagged, key=lambda r: r.cp):
        out.append(
            f"| {_fmt(r.cp)} | {r.name.title()} | {r.cls} | {r.category} | {r.gdef} "
            f"| {r.used_by} | {r.note} |"
        )
    REVIEW_PATH.parent.mkdir(parents=True, exist_ok=True)
    REVIEW_PATH.write_text("\n".join(out) + "\n", encoding="utf-8")


def main(argv: list[str]) -> int:
    if argv[1:] == ["generate"]:
        write_table(generate())
        return 0
    if argv[1:] == ["review"]:
        write_review(load_table())
        return 0
    table = load_table()
    evidence = collect_evidence()
    stale = [
        _fmt(cp)
        for cp, ev in evidence.items()
        if cp not in table
        or (table[cp].category, table[cp].name, table[cp].gdef, table[cp].used_by) != ev
    ]
    extra = [_fmt(cp) for cp in table if cp not in evidence]
    for msg in [*(f"stale or missing: {c}" for c in stale), *(f"unused row: {c}" for c in extra)]:
        print(msg, file=sys.stderr)
    return 1 if stale or extra else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
