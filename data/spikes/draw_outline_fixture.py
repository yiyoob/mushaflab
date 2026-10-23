"""T3.3: independent outline expectations for the core's CFF2 extraction (D4).

    uv run --directory data python -m spikes.draw_outline_fixture

fontTools draws every 25th glyph of the tracked DigitalKhatt Madina font with a RecordingPen;
coordinates are written in 1/64 font unit (rounded), the core's 26.6. The Rust test allows one
unit of rounding difference (fontTools evaluates CFF2 in floating point, the core in 16.16).
"""

import hashlib
import json
import sys

from fontTools.pens.recordingPen import RecordingPen
from fontTools.ttLib import TTFont

from spikes.common import REPO

FONT = REPO / "fonts" / "digitalkhatt" / "hb-madina.otf"
OUT = REPO / "tests" / "golden" / "draw" / "outline-sample.json"
STEP = 25


def main() -> int:
    font = TTFont(FONT)
    gs, order = font.getGlyphSet(), font.getGlyphOrder()
    cases = []
    for gid in range(0, len(order), STEP):
        pen = RecordingPen()
        gs[order[gid]].draw(pen)
        ops = []
        for op, args in pen.value:
            pts = [[round(x * 64), round(y * 64)] for x, y in args]
            ops.append([op, pts])
        cases.append({"glyph": gid, "ops": ops})
    doc = {
        "font_sha256": hashlib.sha256(FONT.read_bytes()).hexdigest(),
        "glyphs": len(order),
        "cases": cases,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    body = ",\n".join(json.dumps(c, separators=(",", ":")) for c in doc.pop("cases"))
    head = json.dumps(doc, indent=1)[:-2]
    OUT.write_text(f'{head},\n "cases": [\n{body}\n ]\n}}\n', encoding="ascii")
    print(f"draw_outline_fixture: {len(order) // STEP + 1} glyphs -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
