"""T3.2: the 1441H print's own widths of its short lines (D15, G-020).

`quran-qcf4` (pinned, 1441H, the edition of the spine and layout) gives every word of every
line as a glyph of KFGQPC's QCF4 page fonts, traced from the print. A line's printed width is
the sum of its word and ayah-number glyph advances (the hizb sign is drawn on its own and not
counted). Full lines vary by about +-3% around their median; a line is **short** when its
width is under SHORT of that median. Each short line gets its ratio to the median, in parts per
million, written to data/render/uthmani-1441h.line-widths.json. Lines the layout marks
`centered` and lines this rule finds short must agree; every disagreement is listed in the
file (`centered_disagreements`) for the owner, never resolved silently.

    python -m pipeline.line_widths build   # write the file
    python -m pipeline.line_widths check   # exit 1 if stale
"""

import io
import json
import statistics
import sys
import tarfile
from pathlib import Path

from fontTools.ttLib import TTFont

from pipeline import sources
from pipeline.ingest_layout import LAYOUT_ID, QCF4, load_layout

# data/render/, not data/layouts/: the loaders read every layouts/*.json as a layout.
OUT = Path(__file__).resolve().parents[1] / "render" / f"{LAYOUT_ID}.line-widths.json"
SHORT_PPM = 900_000  # under 90% of a median full line (quran-madina-html uses the same cut)
COUNTED = ("word", "end")


def qcf4_line_advances() -> dict[tuple[int, int], int]:
    """(page, line) -> sum of glyph advances of the line's words and ayah numbers (QCF4 units)."""
    out: dict[tuple[int, int], int] = {}
    with tarfile.open(sources.path(QCF4)) as tf:
        fonts: dict[str, tuple[dict, object]] = {}

        def advance(font: str, code: int) -> int:
            if font not in fonts:
                member = tf.extractfile(f"package/fonts-woff2/{font}_W.woff2")
                if member is None:
                    raise ValueError(f"qcf4 font {font} missing")
                t = TTFont(io.BytesIO(member.read()))
                fonts[font] = (t.getBestCmap(), t["hmtx"])
            cmap, hmtx = fonts[font]
            return hmtx[cmap[code]][0]  # type: ignore[index]

        for page in range(1, 605):
            member = tf.extractfile(f"package/pages/{page:03d}.json")
            if member is None:
                raise ValueError(f"qcf4 page {page} missing")
            for line in json.load(member)["lines"]:
                words = [w for w in line["words"] if w["type"] in COUNTED]
                if words:
                    out[(page, line["line"])] = sum(advance(w["font"], w["code"]) for w in words)
    return out


def build() -> dict:
    layout = {(r["page"], r["line"]): r for r in load_layout()["lines"]}
    adv = qcf4_line_advances()
    text_lines = {k for k, r in layout.items() if r["type"] == "text"}
    if set(adv) != text_lines:
        raise ValueError("qcf4 text lines differ from the layout's text lines")
    full = statistics.median(v for k, v in adv.items() if not layout[k]["centered"])
    short, disagree = [], []
    for (page, line), v in sorted(adv.items()):
        ppm = round(v * 1_000_000 / full)
        is_short = ppm < SHORT_PPM
        if is_short:
            short.append({"page": page, "line": line, "width_ppm": ppm})
        if is_short != layout[(page, line)]["centered"]:
            disagree.append(
                {"page": page, "line": line, "width_ppm": ppm,
                 "centered": layout[(page, line)]["centered"]}
            )  # fmt: skip
    manifest = {s.id: s.sha256 for s in sources.load_manifest(sources.MANIFEST_PATH).sources}
    return {
        "layout_id": LAYOUT_ID,
        "source": {"id": QCF4, "sha256": manifest[QCF4]},
        "rule": f"short when the printed width is under {SHORT_PPM} ppm of the median full line",
        "median_full_line_qcf4_units": full,
        "short_lines": short,
        "centered_disagreements": disagree,
    }


def text(data: dict) -> str:
    rows = data.pop("short_lines")
    head = json.dumps(data, ensure_ascii=True, indent=1)[:-2]
    body = ",\n".join("  " + json.dumps(r) for r in rows)
    return f'{head},\n "short_lines": [\n{body}\n ]\n}}\n'


def main(argv: list[str]) -> int:
    cmd = argv[1] if len(argv) > 1 else ""
    if cmd not in ("build", "check"):
        print(__doc__, file=sys.stderr)
        return 2
    out = text(build())
    if cmd == "check":
        ok = OUT.exists() and OUT.read_text(encoding="ascii") == out
        if not ok:
            print(f"line_widths: stale {OUT}", file=sys.stderr)
        return 0 if ok else 1
    Path(OUT).write_text(out, encoding="ascii")
    print(f"line_widths: {out.count('width_ppm')} entries -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
