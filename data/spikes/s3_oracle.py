"""T3.2: oracle expectations for the Rust justifier (D3), without text.

    uv run --with numpy --with freetype-py --directory data python -m spikes.s3_oracle

Runs the quran-madina-html Python port of DigitalKhatt's justifier (pinned commit, cloned to
/tmp, rule 10) on every `ayah` line of DigitalKhatt's own Madina text, at DigitalKhatt's own
line widths (`newMadinaLineWidths`, page geometry 16200 x (1 - 2 x 400 / 17000) font units),
with the reference settings: spaces may grow `space_stretch` x normal before letters stretch,
and the justifier stops at the first overflow (not exhaustive). Only lines narrower than their
target are recorded: the Rust port's overflow policy is its own (D15), not the oracle's.

The target width is quantized to 1/64 font unit (26.6, D10) before the oracle runs, so both
sides justify to exactly the same target. Writes tests/golden/justify/oracle-1421h.json:
per line the page, line, target (26.6), natural width, every stretch feature as
[char index in line, tag, value], and the two space widths (26.6, rounded).
"""

import json
import sys

import uharfbuzz as hb

from spikes.common import REPO, qul_layout, verified_sources
from spikes.s1_editions import dk_text_pages
from spikes.s3_justify import FULL, oracle, width_ratios

OUT = REPO / "tests" / "golden" / "justify" / "oracle-1421h.json"
# font None = the pinned DigitalKhatt font; "derived" = the D17 render font
DERIVED_FONT = REPO / "fonts" / "digitalkhatt" / "hb-madina-mushaflab.otf"
SETTINGS = (
    {"space_stretch": 1.0, "pages": None, "font": None},
    {"space_stretch": 0.25, "pages": (3, 4, 50), "font": None},
    {"space_stretch": 1.0, "pages": None, "font": "derived"},
)


def run(j, font: hb.Font, space_stretch: float, pages_wanted: tuple | None) -> list[dict]:  # noqa: ANN001
    types = {(r[0], r[1]): r[2] for r in qul_layout("qul-layout-10-v2-1421h")}
    ratios = width_ratios()
    out = []
    for pno, page in enumerate(dk_text_pages("digitalkhatt-text-madina"), 1):
        if pages_wanted and pno not in pages_wanted:
            continue
        for lno, text in enumerate(page, 1):
            if types[(pno, lno)] != "ayah":
                continue
            target = round(FULL * ratios.get((pno, lno), 1.0) * 64)
            res = j.justify_line(font, text, target / 64, space_stretch=space_stretch)
            if res.natural_width >= target / 64:
                continue
            feats = sorted([i, tag, v] for i, fs in res.features.items() for tag, v in fs.items())
            out.append(
                {
                    "page": pno,
                    "line": lno,
                    "target": target,
                    "natural": round(res.natural_width),
                    "features": feats,
                    "simple": round(res.simple_spacing * 64),
                    "aya": round(res.aya_spacing * 64),
                }
            )
    return out


def main() -> int:
    j = oracle()
    src = verified_sources()
    fonts = {
        None: hb.Font(hb.Face(hb.Blob.from_file_path(str(src["digitalkhatt-hb-madina-font"])))),
        "derived": hb.Font(hb.Face(hb.Blob.from_file_path(str(DERIVED_FONT)))),
    }
    runs = []
    for s in SETTINGS:
        lines = run(j, fonts[s["font"]], s["space_stretch"], s["pages"])
        runs.append(
            {
                "space_stretch_per_mille": round(s["space_stretch"] * 1000),
                "pages": list(s["pages"]) if s["pages"] else "all",
                "font": s["font"] or "pinned",
                "lines": lines,
            }
        )
        print(
            f"space_stretch {s['space_stretch']}, font {s['font'] or 'pinned'}: {len(lines)} lines"
        )
    head = {
        "note": "quran-madina-html justify.justify_line (oracle, @ d520c5c) on DigitalKhatt's "
        "Madina text; lines keyed by DigitalKhatt page/line; no text. Widths: target, simple, "
        "aya in 1/64 font unit; natural in font units.",
        "harfbuzz": hb.version_string(),
        "line_width": FULL,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    parts = []
    for r in runs:
        lines = ",\n".join(json.dumps(x, separators=(",", ":")) for x in r.pop("lines"))
        parts.append(json.dumps(r, separators=(",", ":"))[:-1] + ',"lines":[\n' + lines + "\n]}")
    text = json.dumps(head, indent=1)[:-2] + ',\n "runs": [\n' + ",\n".join(parts) + "\n ]\n}\n"
    OUT.write_text(text, encoding="ascii")
    return 0


if __name__ == "__main__":
    sys.exit(main())
