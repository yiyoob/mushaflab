"""D17 (G-045): the render font, derived from the pinned DigitalKhatt Madina font by one patch.

DigitalKhatt Madina's `rclt` feature maps every U+0670 (superscript alef) its contextual
lookups left alone to `glyph00103`, a spacing upright alef; after non-joining letters that puts
an alef in a gap where the 1441H print draws a superscript mark (owner report at GATE G1,
2026-10-22). The patch changes that one single substitution to `glyph00102`, one of the font's
own anchored superscript-alef marks. Nothing else changes except the names, which say the font
is modified (SIL OFL 1.1 modified version; no Reserved Font Name is declared).

The patch is checked, not assumed: exactly one single substitution in the whole GSUB maps
uni0670 to glyph00103, it belongs to a lookup of the `rclt` feature, and glyph00102 is a GDEF
mark. Untouched tables (outlines included) are copied byte for byte. The output is
deterministic (no timestamp is recalculated) and its sha256 is recorded.

    python -m pipeline.derived_font write   # write fonts/digitalkhatt/hb-madina-mushaflab.otf
    python -m pipeline.derived_font check   # exit 1 if the tracked file differs
"""

import hashlib
import io
import json
import sys
from pathlib import Path

import fontTools
from fontTools.ttLib import TTFont

from pipeline import sources

REPO = Path(__file__).resolve().parents[2]
SOURCE = "digitalkhatt-hb-madina-font"
OUT = REPO / "fonts" / "digitalkhatt" / "hb-madina-mushaflab.otf"
RECORD = OUT.with_suffix(".json")
FEATURE, GLYPH, FROM, TO = "rclt", "uni0670", "glyph00103", "glyph00102"
MARK_CLASS = 3
FAMILY = "DigitalKhatt New Madina Mushaflab"
NAMES = {
    1: FAMILY,
    3: f"{FAMILY} Regular; mushaflab D17",
    4: f"{FAMILY} Regular",
    5: "Version 0.1; mushaflab D17 patch 1 (rclt: uni0670 -> glyph00102)",
    6: "DigitalKhattNewMadinaMushaflab-Regular",
    16: FAMILY,
}


class PatchError(ValueError):
    pass


def _single_subst(lookup) -> list:  # noqa: ANN001 - fontTools lookup object
    tables = []
    for st in lookup.SubTable:
        inner = st.ExtSubTable if lookup.LookupType == 7 else st  # noqa: PLR2004 - extension
        if isinstance(getattr(inner, "mapping", None), dict):
            tables.append(inner)
    return tables


def patch(font: TTFont) -> dict:
    """Apply the D17 patch in place; return what was changed."""
    gsub = font["GSUB"].table
    hits = [
        (i, st)
        for i, lk in enumerate(gsub.LookupList.Lookup)
        for st in _single_subst(lk)
        if st.mapping.get(GLYPH) == FROM
    ]
    if len(hits) != 1:
        raise PatchError(f"expected one {GLYPH} -> {FROM} substitution, found {len(hits)}")
    index, st = hits[0]
    rclt = {
        li
        for fr in gsub.FeatureList.FeatureRecord
        if fr.FeatureTag == FEATURE
        for li in fr.Feature.LookupListIndex
    }
    if index not in rclt:
        raise PatchError(f"lookup {index} is not a {FEATURE} lookup")
    classes = font["GDEF"].table.GlyphClassDef.classDefs
    if classes.get(TO) != MARK_CLASS:
        raise PatchError(f"{TO} is not a GDEF mark")
    st.mapping[GLYPH] = TO
    for rec in font["name"].names:
        if rec.nameID in NAMES:
            rec.string = NAMES[rec.nameID]
    return {"lookup": index, "feature": FEATURE, "glyph": GLYPH, "from": FROM, "to": TO}


def build() -> tuple[bytes, dict]:
    src = sources.path(SOURCE)
    font = TTFont(src, recalcTimestamp=False, recalcBBoxes=False)
    change = patch(font)
    buf = io.BytesIO()
    font.save(buf)
    data = buf.getvalue()
    manifest = {s.id: s.sha256 for s in sources.load_manifest(sources.MANIFEST_PATH).sources}
    record = {
        "path": str(OUT.relative_to(REPO)),
        "derived_from": {"source": SOURCE, "sha256": manifest[SOURCE]},
        "decision": "D17 (G-045), owner 2026-10-22",
        "patch": change,
        "names": {str(k): v for k, v in NAMES.items()},
        "fonttools": fontTools.version,
        "sha256": hashlib.sha256(data).hexdigest(),
    }
    return data, record


def record_text(record: dict) -> str:
    return json.dumps(record, indent=1, ensure_ascii=True) + "\n"


def main(argv: list[str]) -> int:
    cmd = argv[1] if len(argv) > 1 else ""
    if cmd not in ("write", "check"):
        print(__doc__, file=sys.stderr)
        return 2
    data, record = build()
    if cmd == "check":
        ok = (
            OUT.exists()
            and OUT.read_bytes() == data
            and RECORD.read_text(encoding="ascii") == record_text(record)
        )
        if not ok:
            print(f"derived_font: {OUT} or its record is stale", file=sys.stderr)
        return 0 if ok else 1
    OUT.write_bytes(data)
    RECORD.write_text(record_text(record), encoding="ascii")
    print(f"derived_font: {OUT} sha256 {record['sha256']}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
