"""Spike S3 (T0.5): justification feasibility.

    uv run --with numpy --with freetype-py --directory data python -m spikes.s3_justify

Uses the quran-madina-html justifier (Python port of DigitalKhatt's MIT justifier) as the
oracle. It is a reference repo cloned to /tmp (CLAUDE.md rule 10), pinned to ORACLE_COMMIT,
and never vendored. Outputs go to build/s3/ (untracked):
  * cases.json: per word, the oracle's stretch features and HarfBuzz's exact glyph output,
    for the Rust harfrust comparison (core/spikes/s3_harfrust);
  * a report on stdout.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

import uharfbuzz as hb

from spikes.common import REPO, qul_layout, verified_sources
from spikes.s1_editions import dk_text_pages

# CLAUDE.md rule 10: third-party reference repos are cloned to /tmp, never vendored.
ORACLE_DIR = Path("/tmp/mushaflab-ref/quran-madina-html")  # noqa: S108
ORACLE_COMMIT = "d520c5cf81a05ed92db8861c692104d4b5455c13"
OUT_DIR = REPO / "build" / "s3"
PAGES = (3, 4, 50, 106, 187, 255, 300, 377, 442, 528, 582, 604)
SPACE_STRETCH = 0.25  # spaces may grow 25% before letters stretch (exercises cvNN features)
FULL = 16200 * (1 - 2 * 400 / 17000)  # DigitalKhatt 1441H/1421H page geometry, font units
SMALL_HIGH_SEEN = "\u06dc"


def oracle():  # noqa: ANN201 - returns the oracle module
    head = subprocess.run(
        ["git", "-C", str(ORACLE_DIR), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if head != ORACLE_COMMIT:
        raise SystemExit(f"oracle at {head}, expected {ORACLE_COMMIT}")
    sys.path.insert(0, str(ORACLE_DIR / "src" / "db" / "v2"))
    import justify  # noqa: PLC0415 - reference module from the pinned /tmp clone

    return justify


def width_ratios() -> dict[tuple[int, int], float]:
    svc = verified_sources()["digitalkhatt-line-widths"].read_text(encoding="utf-8")
    block = re.search(r"newMadinaLineWidths = new Map\(\[(.*?)\]\);", svc, re.DOTALL)
    if block is None:
        raise SystemExit("newMadinaLineWidths not found")
    pairs = re.findall(r"\[(\d+) \* 15 \+ (\d+), ([\d.]+)\]", block.group(1))
    # DigitalKhatt keys are 0-based page and line indices.
    return {(int(p) + 1, int(n) + 1): float(r) for p, n, r in pairs}


def glyphs(font: hb.Font, text: str, features: dict) -> list[list[int]]:
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    buf.cluster_level = hb.BufferClusterLevel.MONOTONE_CHARACTERS
    hb.shape(font, buf, features)
    return [
        [i.codepoint, i.cluster, p.x_advance, p.x_offset, p.y_offset]
        for i, p in zip(buf.glyph_infos, buf.glyph_positions, strict=True)
    ]


def run_lines(j, font: hb.Font) -> tuple[list[dict], list[tuple]]:  # noqa: ANN001
    types = {(r[0], r[1]): r[2] for r in qul_layout("qul-layout-10-v2-1421h")}
    ratios = width_ratios()
    pages = dk_text_pages("digitalkhatt-text-madina")
    cases, stats = [], []
    for pno in PAGES:
        for lno, text in enumerate(pages[pno - 1], 1):
            if types[(pno, lno)] != "ayah":
                continue
            desired = FULL * ratios.get((pno, lno), 1.0)
            res = j.justify_line(font, text, desired, space_stretch=SPACE_STRETCH)
            info = j.analyze_line(text)
            stretched = 0
            for word in info.words:
                feats = j.to_hb_features(res.features, word.start, word.start, word.end)
                stretched += bool(feats)
                cases.append(
                    {
                        "page": pno,
                        "line": lno,
                        "text": word.text,
                        "features": [
                            [tag, s, e, v] for tag, ranges in feats.items() for s, e, v in ranges
                        ],
                        "glyphs": glyphs(font, word.text, feats),
                    }
                )
            stats.append(
                (pno, lno, res.natural_width, desired, res.final_width, res.x_scale, stretched)
            )
    return cases, stats


def edit_demo(j, font: hb.Font) -> None:  # noqa: ANN001
    """Synthetic free-mode edits (test fixtures, not Quranic data) on page 3, line 5:
    add U+06DC to the longest word, then insert one extra copy of that word's first letter."""
    pages = dk_text_pages("digitalkhatt-text-madina")
    text = pages[2][4]
    words = text.split(" ")
    target = max(range(len(words)), key=lambda i: len(words[i]))
    with_mark = list(words)
    with_mark[target] = words[target] + SMALL_HIGH_SEEN
    with_letter = list(words)
    with_letter[target] = words[target][0] + words[target]
    for label, ws in (
        ("original", words),
        ("+ U+06DC mark", with_mark),
        ("+ one letter", with_letter),
    ):
        t = " ".join(ws)
        res = j.justify_line(font, t, FULL, space_stretch=SPACE_STRETCH)
        print(
            f"| {label} | {j.width(font, ws[target])} | {res.natural_width:.0f} "
            f"| {res.final_width:.0f} | {sum(len(v) for v in res.features.values())} "
            f"| {res.simple_spacing:.1f} |"
        )


def main() -> int:
    j = oracle()
    src = verified_sources()
    font = hb.Font(hb.Face(hb.Blob.from_file_path(str(src["digitalkhatt-hb-madina-font"]))))
    cases, stats = run_lines(j, font)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "cases.json").write_text(
        json.dumps(
            {
                "font": str(src["digitalkhatt-hb-madina-font"]),
                "harfbuzz": hb.version_string(),
                "cases": cases,
            },
            ensure_ascii=True,
        ),
        encoding="utf-8",
    )
    closed = sum(1 for s in stats if abs(s[4] - s[3]) <= 0.001 * s[3])
    shrunk = sum(1 for s in stats if s[2] > s[3])
    print(f"HarfBuzz {hb.version_string()}; pages {PAGES}; space_stretch {SPACE_STRETCH}")
    print(f"Lines: {len(stats)}; closed within 0.1%: {closed}; naturally too wide: {shrunk}")
    with_features = sum(1 for c in cases if c["features"])
    print(f"Words: {len(cases)}; words with stretch features: {with_features}")
    print()
    print("Edit demo (page 3 line 5, longest word; synthetic test edits):")
    print()
    print("| Line | Word width | Natural line | Final line | Stretch features | Space |")
    print("|---|---|---|---|---|---|")
    edit_demo(j, font)
    spacing_only_section()
    return 0


def spacing_only_section() -> None:
    """How far would word spacing alone have to stretch, per font/layout pairing?

    For every full-width ayah line, shape its words and measure the natural width with
    normal spaces. The target is the widest natural line in the pairing (so no line has
    to shrink); the needed space ratio is the space width each line would need to reach
    it, relative to a normal space. Short lines (DigitalKhatt width ratios, surah ends)
    are excluded.
    """
    import statistics  # noqa: PLC0415

    from spikes.common import extracted, kfgqpc_official, only  # noqa: PLC0415
    from spikes.s1_editions import qul_words  # noqa: PLC0415

    def font_of(path: Path) -> hb.Font:
        return hb.Font(hb.Face(hb.Blob.from_file_path(str(path))))

    def w(font: hb.Font, text: str) -> int:
        return sum(p.x_advance for _, p in [(0, q) for q in shape_positions(font, text)])

    def shape_positions(font: hb.Font, text: str) -> list:
        buf = hb.Buffer()
        buf.add_str(text)
        buf.guess_segment_properties()
        hb.shape(font, buf, {})
        return buf.glyph_positions

    src = verified_sources()
    ratios = width_ratios()
    results = {}
    # DigitalKhatt Madina, its own 1421H text and lines.
    dk_font = font_of(src["digitalkhatt-hb-madina-font"])
    types = {(r[0], r[1]): r[2] for r in qul_layout("qul-layout-10-v2-1421h")}
    lines = []
    for pno, page in enumerate(dk_text_pages("digitalkhatt-text-madina"), 1):
        for lno, text in enumerate(page, 1):
            if types[(pno, lno)] == "ayah" and (pno, lno) not in ratios and pno > 2:
                lines.append(text.split(" "))
    results["DigitalKhatt Madina + 1421H lines"] = (dk_font, lines)
    # KFGQPC Hafs v20 font, 1441H lines (QUL V4) with the QUL KFGQPC word texts.
    kf_font = font_of(only(sorted(extracted("kfgqpc-official-hafs").glob("*/*.ttf"))))
    words = qul_words()
    lines = []
    for page, _line, kind, _c, first, last, _s in qul_layout("qul-layout-19-v4-1441h"):
        if kind == "ayah" and page > 2:
            lines.append([words[i][3] for i in range(first, last + 1)])
    results["KFGQPC Hafs v20 + 1441H lines"] = (kf_font, lines)
    _ = kfgqpc_official  # text source documented in the report; QUL words carry the layout ids
    print()
    print("Target = 90th-percentile natural width; lines under 70% of it count as short.")
    print()
    print("| Pairing | Full lines | Need shrink | Space ratio median | p95 | max |")
    print("|---|---|---|---|---|---|")
    for name, (font, lines) in results.items():
        space = w(font, " ")
        nat = [sum(w(font, x) for x in ln) + space * (len(ln) - 1) for ln in lines]
        target = sorted(nat)[int(0.9 * (len(nat) - 1))]
        full = [(n, ln) for n, ln in zip(nat, lines, strict=True) if n >= 0.7 * target]
        wide = sum(1 for n, _ in full if n > target)
        need = sorted(
            (target - n) / max(1, len(ln) - 1) / space + 1 for n, ln in full if n <= target
        )
        p95 = need[int(0.95 * (len(need) - 1))]
        print(
            f"| {name} | {len(full)} | {wide} | {statistics.median(need):.2f} "
            f"| {p95:.2f} | {need[-1]:.2f} |"
        )


if __name__ == "__main__":
    sys.exit(main())
