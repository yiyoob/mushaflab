"""T3.1: turn the S3 harfrust/HarfBuzz parity set into a regression golden without text.

    uv run --with numpy --with freetype-py --directory data python -m spikes.s3_justify
    uv run --directory data python -m spikes.s3_parity_fixture

Reads build/s3/cases.json (HarfBuzz output per word, written by spikes.s3_justify) and writes
tests/golden/shape/s3-parity.json, where each case names its word by DigitalKhatt page, line
and word index instead of carrying the text (rule 1: no Quranic text copied into tests; the
Rust test reads the word from the pinned `digitalkhatt-text-madina` source).
"""

import json
import sys

from pipeline import sources
from spikes.common import REPO
from spikes.s1_editions import dk_text_pages

CASES = REPO / "build" / "s3" / "cases.json"
OUT = REPO / "tests" / "golden" / "shape" / "s3-parity.json"
DK_TEXT, DK_FONT = "digitalkhatt-text-madina", "digitalkhatt-hb-madina-font"


def main() -> int:
    data = json.loads(CASES.read_text(encoding="utf-8"))
    pages = dk_text_pages(DK_TEXT)
    manifest = {s.id: s.sha256 for s in sources.load_manifest(sources.MANIFEST_PATH).sources}
    out, cursor = [], {}
    for case in data["cases"]:
        key = (case["page"], case["line"])
        words = pages[key[0] - 1][key[1] - 1].split(" ")
        index = cursor.get(key, 0)
        while words[index] != case["text"]:
            index += 1
        cursor[key] = index + 1
        out.append(
            {
                "page": key[0],
                "line": key[1],
                "word": index,
                "features": case["features"],
                "glyphs": case["glyphs"],
            }
        )
    doc = {
        "note": "HarfBuzz output for the S3 word set (T0.5), keyed by DigitalKhatt page/line/"
        "word index (0-based, split on U+0020). Glyph: [id, cluster, x_advance, x_offset, "
        "y_offset]; feature: [tag, start, end, value], char indices within the word.",
        "harfbuzz": data["harfbuzz"],
        "sources": {DK_TEXT: manifest[DK_TEXT], DK_FONT: manifest[DK_FONT]},
        "cases": out,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    lines = ",\n".join(json.dumps(c, separators=(",", ":")) for c in doc.pop("cases"))
    head = json.dumps(doc, ensure_ascii=True, indent=1)[:-2]
    OUT.write_text(f'{head},\n "cases": [\n{lines}\n ]\n}}\n', encoding="ascii")
    print(f"s3_parity_fixture: {len(out)} words -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
