"""T5.1: riwayah profiles (`data/profiles/<riwayah>.yaml`).

    python -m pipeline.profiles write   # rebuild the 8 profiles
    python -m pipeline.profiles check   # exit 1 if a tracked profile is stale

A profile selects what the resolver needs for one riwayah: its text source, its patch stack
on the Hafs spine, its ayah map, and its surah transitions as the source lays them out.

Surah transitions are data, not knowledge: before each surah's first ayah, the riwayah's
KFGQPC file leaves some lines without an ayah. Each such line takes the kind (surah name or
basmalah) of the 1441H Hafs layout's line at the same page and line. A gap line that Hafs
uses for text is accepted only from the listed hypotheses (`HYPOTHESES`), which the owner
checks against the print; anything else is an error. Recitation choices between surahs
(basmalah, sakt, wasl) are option groups (Phase 6), not part of the page.
"""

import sys
from pathlib import Path

from pipeline import emit_patches, ingest_layout, ingest_riwayah, kfgqpc
from pipeline.emit_patches import flow

PROFILE_DIR = Path(__file__).resolve().parents[1] / "profiles"
SCRIPT_ID = "uthmani"

G023 = (
    "hypothesis (G-023): the KFGQPC file leaves this line without an ayah where Hafs has "
    "its numbered basmalah; drawn as an unnumbered basmalah line until the owner checks the "
    "print"
)
# (surah, page, line) of gap lines that Hafs uses for text, with their stated kind.
HYPOTHESES = {(1, 1, 2): ("basmalah", G023)}

Pos = tuple[int, int]


class ProfileError(ValueError):
    pass


def path_for(riwayah: str) -> Path:
    return PROFILE_DIR / f"{riwayah}.yaml"


def lines_per_page() -> dict[int, int]:
    """Lines on each page of the 1441H layout (pages 1 and 2 are shorter)."""
    out: dict[int, int] = {}
    for ln in ingest_layout.load_layout()["lines"]:
        out[ln["page"]] = max(out.get(ln["page"], 0), ln["line"])
    return out


def gap_lines(rows: list[dict], lines: dict[int, int]) -> dict[int, list[Pos]]:
    """Per surah, the (page, line) positions between the previous surah's last ayah line and
    the surah's first ayah line, as the riwayah's KFGQPC rows place them."""
    out: dict[int, list[Pos]] = {}
    end: Pos = (1, 0)
    seen: set[int] = set()
    for r in rows:
        first_page, last_page = ingest_riwayah.parse_page(r["page"])
        start = (first_page, int(r["line_start"]))
        sura = int(r["sura_no"])
        if sura not in seen:
            seen.add(sura)
            gap: list[Pos] = []
            page, line = end
            while (page, line) != start:
                line += 1
                if line > lines[page]:
                    page, line = page + 1, 1
                if (page, line) != start:
                    gap.append((page, line))
                if page > start[0]:
                    raise ProfileError(f"surah {sura}: start {start} is not after {end}")
            out[sura] = gap
        end = (last_page, int(r["line_end"]))
    return out


def headers(gaps: dict[int, list[Pos]], hafs: dict[Pos, str]) -> list[dict]:
    out = []
    for sura, gap in sorted(gaps.items()):
        pages = {p for p, _ in gap}
        if len(pages) != 1:
            raise ProfileError(f"surah {sura}: header lines {gap} are not on one page")
        page = pages.pop()
        kinds = []
        for p, n in gap:
            kind = hafs.get((p, n))
            if kind == "text" and (sura, p, n) in HYPOTHESES:
                kinds.append(HYPOTHESES[(sura, p, n)])
            elif kind in ("surah_name", "basmalah"):
                kinds.append((kind, None))
            else:
                raise ProfileError(f"surah {sura}: line {p}:{n} has no header kind ({kind})")
        if [k for k, _ in kinds] not in (["surah_name"], ["surah_name", "basmalah"]):
            raise ProfileError(f"surah {sura}: header lines {kinds} are not name [+ basmalah]")
        h: dict = {"surah": sura, "page": page, "name_line": gap[0][1]}
        if len(gap) > 1:
            h["basmalah_line"] = gap[1][1]
            if kinds[1][1]:
                h["basmalah_evidence"] = kinds[1][1]
        out.append(h)
    return out


def hafs_kinds() -> dict[Pos, str]:
    return {(ln["page"], ln["line"]): ln["type"] for ln in ingest_layout.load_layout()["lines"]}


def build(riwayah: str) -> dict:
    rows = kfgqpc.rows(riwayah)
    p: dict = {
        "riwayah": riwayah,
        "spine": emit_patches.SPINE_ID,
        "script_id": SCRIPT_ID,
        "text_source": kfgqpc.source_id(riwayah),
    }
    if riwayah == "hafs":
        p["patch_stack"] = []
    else:
        p["single_source"] = True
        p["patch_stack"] = [f"patches/raw/{riwayah}.yaml"]
        p["ayah_map"] = riwayah
    p["surah_headers"] = headers(gap_lines(rows, lines_per_page()), hafs_kinds())
    return p


def render(riwayah: str) -> str:
    p = build(riwayah)
    lines = [
        f"# Riwayah profile for {riwayah} (T5.1). Surah headers: lines without an ayah in "
        f"{p['text_source']},",
        "# typed by the 1441H Hafs layout. Generated by `python -m pipeline.profiles write`. "
        "Do not edit.",
    ]
    for key, value in p.items():
        if key == "surah_headers":
            lines.append("surah_headers:")
            lines += ["  - " + flow(h) for h in value]
        else:
            lines.append(f"{key}: {flow(value)}")
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    cmd = argv[1] if len(argv) > 1 else "write"
    stale = []
    for r in kfgqpc.RIWAYAT:
        text, path = render(r), path_for(r)
        if cmd == "write":
            PROFILE_DIR.mkdir(exist_ok=True)
            path.write_text(text, encoding="ascii")
        elif not path.exists() or path.read_text(encoding="ascii") != text:
            stale.append(path.name)
    for name in stale:
        print(f"profiles: {name} differs from a rebuild", file=sys.stderr)
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
