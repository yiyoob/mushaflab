"""M1.3: corpus/adad/kufi.tsv and corpus/layouts/uthmani-1441h/ equal the legacy tables.

Expected values come only from tracked files: the Hafs spine (its items and manifest, built
from the pinned KFGQPC text) and data/layouts/uthmani-1441h.json (rule 1).
"""

import json

from pipeline import corpus_layout, spine


def _body(path) -> list[list[str]]:
    lines = path.read_text(encoding="utf-8").split("\n")
    rows = [ln.split("\t") for ln in lines if ln and not ln.startswith("#")]
    return rows


def test_tracked_tables_are_current() -> None:
    for path, render in corpus_layout.OUTPUTS.items():
        assert path.read_text(encoding="utf-8") == render(), path


def test_every_ayah_equals_the_spine() -> None:
    rows = _body(corpus_layout.ADAD_OUT)
    assert rows[0] == corpus_layout.ADAD_HEADER
    by_ayah: dict[tuple[int, int], list[str]] = {}
    ends: dict[tuple[int, int], str] = {}
    for it in corpus_layout._items():
        s, a, _ = (int(x) for x in it["id"].split(":"))
        if it["kind"] == "word":
            by_ayah.setdefault((s, a), []).append(it["id"])
        elif it["kind"] == "ayah_end":
            ends[(s, a)] = it["id"]
    manifest = json.loads(spine.MANIFEST.read_text(encoding="utf-8"))
    assert len(rows) - 1 == manifest["counts"]["items"]["ayah_end"] == len(by_ayah)
    for surah, ayah, first, last, source, status in rows[1:]:
        key = (int(surah), int(ayah))
        assert (first, last) == (by_ayah[key][0], by_ayah[key][-1]), key
        assert source == f"kfgqpc-hafs:{ends[key]}"
        assert status == "raw-derived"


def test_every_line_equals_the_legacy_layout() -> None:
    legacy = json.loads(corpus_layout.LEGACY_LAYOUT.read_text(encoding="utf-8"))
    rows = _body(corpus_layout.LINES_OUT)
    assert rows[0] == corpus_layout.LINES_HEADER
    assert len(rows) - 1 == len(legacy["lines"])
    for row, ln in zip(rows[1:], legacy["lines"], strict=True):
        want = [
            str(ln["page"]),
            str(ln["line"]),
            ln["type"],
            ln.get("first", ""),
            ln.get("last", ""),
            "true" if ln["centered"] else "false",
        ]
        assert row == want


def test_layout_file_keeps_the_owner_corrections() -> None:
    corr = json.loads(corpus_layout.LEGACY_CORRECTIONS.read_text(encoding="utf-8"))
    text = corpus_layout.LAYOUT_YAML.read_text(encoding="utf-8")
    for c in corr["corrections"]:
        assert json.dumps(c["evidence"], ensure_ascii=False) in text
