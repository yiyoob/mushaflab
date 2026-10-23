"""T1.5: integrity verification of the spine, the 1441H layout and the DB.

    python -m pipeline.verify                 # run every check; exit 1 on any failure
    python -m pipeline.verify write-golden    # rewrite tests/golden/page_boundaries.json

Expected values come from pinned sources or from agreement between independent sources,
never from memory (rule 1).
"""

import json
import sys
import tarfile
from collections import Counter
from collections.abc import Callable
from itertools import pairwise
from pathlib import Path

from pipeline import (
    align,
    ayah_lines,
    ayah_maps,
    codepoints,
    compile_db,
    dk_riwayat,
    drift,
    emit_patches,
    ingest_layout,
    ingest_riwayah,
    phase4_report,
    profiles,
    sources,
    spine,
)

GOLDEN = Path(__file__).resolve().parents[2] / "tests" / "golden" / "page_boundaries.json"


def _layout() -> dict:
    return ingest_layout.load_layout()


def check_sources() -> list[str]:
    return [] if sources.available() else ["pinned sources missing or not verified"]


def check_tracked_files_current() -> list[str]:
    out = []
    if spine.load_records() != spine.build_records():
        out.append("spine files differ from a rebuild")
    if spine.load_manifest() != spine.build_manifest(spine.load_records()):
        out.append("spine manifest differs from a rebuild")
    if _layout() != ingest_layout.build_layout():
        out.append("layout file differs from a rebuild")
    if codepoints.main(["", "check"]) != 0:
        out.append("codepoint table is stale")
    report = ingest_riwayah.REPORT_PATH
    if not report.exists() or report.read_text(encoding="utf-8") != ingest_riwayah.report():
        out.append("docs/review/phase4/ingest.md differs from a rebuild")
    for path, text in align.build_reports().items():
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            out.append(f"{path.name} differs from a rebuild")
    for r in emit_patches.ORDER:
        path = emit_patches.path_for(r)
        if not path.exists() or path.read_text(encoding="ascii") != emit_patches.render(r):
            out.append(f"patches/raw/{path.name} differs from a rebuild")
    if ayah_maps.main(["", "check"]) != 0:
        out.append("ayah maps or docs/review/phase4/ayah-maps.md differ from a rebuild")
    if phase4_report.main(["", "check"]) != 0:
        out.append("docs/review/phase4/stats.md differs from a rebuild")
    if drift.REPORT_PATH.read_text(encoding="utf-8") != drift.report():
        out.append("docs/review/phase4/version-drift.md differs from a rebuild")
    return out


def check_lines_per_page() -> list[str]:
    """Contiguous line numbers per page, and the same count per page as quran-qcf4."""
    out = []
    mine: dict[int, list[int]] = {}
    for ln in _layout()["lines"]:
        mine.setdefault(ln["page"], []).append(ln["line"])
    for page, numbers in mine.items():
        if numbers != list(range(1, len(numbers) + 1)):
            out.append(f"page {page}: line numbers not contiguous")
    theirs = {}
    with tarfile.open(sources.path(ingest_layout.QCF4)) as tf:
        for page in mine:
            member = tf.extractfile(f"package/pages/{page:03d}.json")
            theirs[page] = len(json.load(member)["lines"]) if member else -1
    out += [
        f"page {p}: {len(n)} lines, qcf4 {theirs[p]}"
        for p, n in mine.items()
        if len(n) != theirs[p]
    ]
    if sorted(mine) != list(range(1, len(mine) + 1)):
        out.append("pages are not numbered 1..n")
    return out


def check_every_item_once_in_order() -> list[str]:
    order = [r["id"] for r in spine.load_records()]
    index = {item_id: n for n, item_id in enumerate(order)}
    covered: list[int] = []
    for ln in _layout()["lines"]:
        if ln["type"] == "text":
            covered.extend(range(index[ln["first"]], index[ln["last"]] + 1))
    if covered == list(range(len(order))):
        return []
    seen = Counter(covered)
    missing = [order[n] for n in range(len(order)) if n not in seen][:5]
    twice = [order[n] for n, c in seen.items() if c > 1][:5]
    return [f"items not exactly once in reading order; missing {missing}, repeated {twice}"]


def check_line_types() -> list[str]:
    out = []
    lines = _layout()["lines"]
    surahs = sorted({int(r["id"].split(":")[0]) for r in spine.load_records()})
    headers = [ln["surah"] for ln in lines if ln["type"] == "surah_name"]
    if headers != surahs:
        out.append("surah-name lines do not match the spine's surahs in order")
    for prev, ln in pairwise(lines):
        if ln["type"] == "basmalah" and prev["type"] != "surah_name":
            out.append(f"basmalah at {ln['page']}:{ln['line']} does not follow a surah name")
    # each surah's first text line must start that surah
    current = None
    for ln in lines:
        if ln["type"] == "surah_name":
            current = ln["surah"]
        elif ln["type"] == "text" and current is not None:
            if int(ln["first"].split(":")[0]) != current or ln["first"].split(":")[1] != "1":
                out.append(f"surah {current}: first text line starts at {ln['first']}")
            current = None
    return out


def check_kfgqpc_pages() -> list[str]:
    return ingest_layout.kfgqpc_page_disagreements()


def check_qcf4_lines() -> list[str]:
    documented = [(92, 12), (92, 13)]  # qcf4's error, confirmed by owner print check (G-011)
    found = ingest_layout.qcf4_disagreements()
    return [] if found == documented else [f"qcf4 disagreements {found}, expected {documented}"]


def check_text_differences() -> list[str]:
    report = ingest_layout.text_report()
    return [f"unexplained text difference at {u}" for u in report["unexplained"]]


def check_db() -> list[str]:
    db = compile_db.DEFAULT_DB
    if not db.exists():
        return ["build/mushaf.db missing: run `just data-build`"]
    out = []
    if compile_db.read_spine(db) != spine.load_records():
        out.append("DB spine differs from the tracked files")
    if compile_db.read_layout(db) != _layout():
        out.append("DB layout differs from the tracked file")
    return out


def check_phase5_files_current() -> list[str]:
    out = []
    if profiles.main(["", "check"]) != 0:
        out.append("data/profiles differ from a rebuild")
    if ayah_lines.main(["", "check"]) != 0:
        out.append("data/layout_inputs differ from a rebuild")
    if dk_riwayat.main(["", "check"]) != 0:
        out.append("riwayat encoding candidates or rows differ from a rebuild")
    return out


CHECKS: dict[str, Callable[[], list[str]]] = {
    "sources": check_sources,
    "tracked files current": check_tracked_files_current,
    "phase 5 files current": check_phase5_files_current,
    "lines per page": check_lines_per_page,
    "every item once, in order": check_every_item_once_in_order,
    "line types": check_line_types,
    "KFGQPC page data": check_kfgqpc_pages,
    "qcf4 line data": check_qcf4_lines,
    "word texts": check_text_differences,
    "database": check_db,
}


def run_checks(checks: dict[str, Callable[[], list[str]]]) -> dict[str, list[str]]:
    failures = {}
    for name, check in checks.items():
        problems = check()
        if problems:
            failures[name] = problems
    return failures


def page_boundaries() -> dict[str, dict]:
    pages: dict[str, dict] = {}
    for ln in _layout()["lines"]:
        page = pages.setdefault(str(ln["page"]), {"lines": 0})
        page["lines"] += 1
        if ln["type"] == "text":
            page.setdefault("first", ln["first"])
            page["last"] = ln["last"]
    return pages


def load_golden() -> dict:
    return json.loads(GOLDEN.read_text(encoding="ascii"))


SPOT_CHECK = Path(__file__).resolve().parents[2] / "docs" / "review" / "spot-check-pages.md"
# Each reason names the item whose page is looked up in the layout (not typed page numbers).
SPOT_CHECK_ITEMS = [
    ("1:1:1", "first page (short page)"),
    ("2:1:1", "second page (short page)"),
    ("2:6:1", "source anomaly: 2:6 starts with a space (G-026)"),
    ("2:286:1", "source anomaly: separator before the ayah sign (G-025)"),
    ("4:91:1", "QUL vs qcf4 disagreement, owner checked (G-011)"),
    ("7:206:1", "sajdah sign in the text"),
    ("9:1:1", "surah without a basmalah line"),
    ("13:37:1", "owner print correction applied (G-011)"),
    ("15:7:1", "KFGQPC word split differently in QUL (G-012)"),
    ("27:20:1", "KFGQPC word split differently in QUL (G-012)"),
    ("36:22:1", "KFGQPC word split differently in QUL (G-012)"),
    ("37:130:1", "QUL joins two words (G-027)"),
    ("18:1:1", "start of surah 18"),
    ("67:1:1", "start of surah 67"),
    ("114:1:1", "last page"),
]


def write_spot_check() -> None:
    def shown(r: dict) -> str:
        if r["kind"] == "ayah_end":
            surah, ayah, _ = r["id"].split(":")
            return f"(end of ayah {surah}:{ayah})"
        return "".join(spine.from_cps(c["cps"]) for c in r["clusters"])

    texts = {r["id"]: shown(r) for r in spine.load_records()}
    lines = _layout()["lines"]
    page_of = {}
    order = [r["id"] for r in spine.load_records()]
    index = {item_id: n for n, item_id in enumerate(order)}
    for ln in lines:
        if ln["type"] == "text":
            for n in range(index[ln["first"]], index[ln["last"]] + 1):
                page_of[order[n]] = ln["page"]
    out = [
        "# Spot check: 15 pages against a printed 1441H mushaf",
        "",
        "Generated by `python -m pipeline.verify spot-check` from the tracked spine and layout.",
        "> **Note (2026-10-22):** this layout follows KFGQPC's digital 1441H data. Printed",
        "> copies that follow the 1421H line layout differ on 9 of these pages (176, 187, 254,",
        "> 262, 293, 378, 441, 451, 562), mostly in which line an ayah marker sits at. The owner",
        "> accepted this (D6 addendum).",
        "",
        "For each line, check that the first and last words match your printed page.",
        "Ayah-end items are shown as `(end of ayah s:a)`; the text stores KFGQPC's ayah sign.",
        "",
    ]
    for item_id, reason in SPOT_CHECK_ITEMS:
        page = page_of[item_id]
        out += [f"## Page {page}: {reason}", "", "| Line | Type | First word | Last word |"]
        out.append("|---|---|---|---|")
        for ln in lines:
            if ln["page"] != page:
                continue
            if ln["type"] == "text":
                first, last = texts[ln["first"]], texts[ln["last"]]
                out.append(
                    f"| {ln['line']} | text | {first} `{ln['first']}` | {last} `{ln['last']}` |"
                )
            else:
                extra = f" (surah {ln['surah']})" if "surah" in ln else ""
                out.append(f"| {ln['line']} | {ln['type']}{extra} | | |")
        out.append("")
    SPOT_CHECK.write_text("\n".join(out), encoding="utf-8")


def main(argv: list[str]) -> int:
    if argv[1:] == ["spot-check"]:
        write_spot_check()
        return 0
    if argv[1:] == ["write-golden"]:
        GOLDEN.write_text(json.dumps(page_boundaries(), indent=1) + "\n", encoding="ascii")
        return 0
    failures = run_checks(CHECKS)
    for name in CHECKS:
        status = "FAIL" if name in failures else "ok"
        print(f"{status:4}  {name}")
        for problem in failures.get(name, [])[:10]:
            print(f"      {problem}")
    if not failures and page_boundaries() != load_golden():
        print("FAIL  page boundaries differ from tests/golden/page_boundaries.json")
        return 1
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
