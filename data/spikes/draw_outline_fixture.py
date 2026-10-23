"""T3.3/T5.3: independent outline expectations for the core's outline extraction (D4).

    uv run --directory data python -m spikes.draw_outline_fixture

fontTools draws every 25th glyph of each tracked font with a RecordingPen; coordinates are
written in 1/64 font unit (rounded), the core's 26.6. DigitalKhatt Madina is CFF2 (the Rust
test allows one unit of rounding difference: fontTools evaluates CFF2 in floating point, the
core in 16.16). The KFGQPC Warsh font is TrueType (T5.3): fontTools' `qCurveTo` runs are
written as single quadratic segments (`quadTo`) with their implied on-curve midpoints, and
composite glyphs are flattened by fontTools in floating point (the Rust test allows two units).
"""

import hashlib
import json
import sys
from pathlib import Path

from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.ttLib import TTFont

from spikes.common import REPO

FONTS = {
    REPO / "fonts" / "digitalkhatt" / "hb-madina.otf": REPO
    / "tests"
    / "golden"
    / "draw"
    / "outline-sample.json",
    REPO / "fonts" / "kfgqpc" / "uthmanic_warsh_v21.ttf": REPO
    / "tests"
    / "golden"
    / "draw"
    / "outline-sample-warsh.json",
}
STEP = 25

Pt = tuple[float, float]


def _mid(a: Pt, b: Pt) -> Pt:
    return ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)


def explicit(value: list) -> list[tuple[str, list[Pt]]]:
    """RecordingPen ops with every `qCurveTo` split into single quadratic segments."""
    out: list[tuple[str, list[Pt]]] = []
    current: Pt | None = None
    for op, args in value:
        if op != "qCurveTo":
            out.append((op, list(args)))
            current = args[-1] if args else current
            continue
        pts = list(args)
        if pts[-1] is None:  # a contour with no on-curve point
            pts = pts[:-1]
            start = _mid(pts[-1], pts[0])
            out.append(("moveTo", [start]))
            current = start
            pts = [*pts, start]
        for k, ctrl in enumerate(pts[:-1]):
            end = pts[-1] if k == len(pts) - 2 else _mid(ctrl, pts[k + 1])
            out.append(("quadTo", [ctrl, end]))
            current = end
    return out


def write(font_path: Path, out_path: Path) -> int:
    font = TTFont(font_path)
    gs, order = font.getGlyphSet(), font.getGlyphOrder()
    cases = []
    for gid in range(0, len(order), STEP):
        pen = DecomposingRecordingPen(gs)
        gs[order[gid]].draw(pen)
        ops = []
        for op, args in explicit(pen.value):
            pts = [[round(x * 64), round(y * 64)] for x, y in args]
            ops.append([op, pts])
        cases.append({"glyph": gid, "ops": ops})
    doc = {
        "font_sha256": hashlib.sha256(font_path.read_bytes()).hexdigest(),
        "glyphs": len(order),
        "cases": cases,
    }
    OUT = out_path  # noqa: N806
    OUT.parent.mkdir(parents=True, exist_ok=True)
    body = ",\n".join(json.dumps(c, separators=(",", ":")) for c in doc.pop("cases"))
    head = json.dumps(doc, indent=1)[:-2]
    OUT.write_text(f'{head},\n "cases": [\n{body}\n ]\n}}\n', encoding="ascii")
    print(f"draw_outline_fixture: {len(order) // STEP + 1} glyphs -> {OUT}")
    return 0


def main() -> int:
    for font_path, out_path in FONTS.items():
        write(font_path, out_path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
