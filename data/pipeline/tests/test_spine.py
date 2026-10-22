"""T1.2: spine files (D11) built from the ingest + clusterizer."""

from collections import Counter
from pathlib import Path

from pipeline import spine
from pipeline.codepoints import load_table
from pipeline.ingest_kfgqpc import Item
from pipeline.tests.markers import needs_sources

BEH, FATHA, NBSP = "\u0628", "\u064e", "\u00a0"


def test_cps_round_trip() -> None:
    text = BEH + FATHA + NBSP
    assert spine.from_cps(spine.to_cps(text)) == text
    assert spine.to_cps("") == ""
    assert spine.to_cps(BEH + FATHA) == "U+0628 U+064E"


def test_record_layout_and_cluster_ids(tmp_path: Path) -> None:
    del tmp_path
    table = {
        0x0628: _row(0x0628, "base"),
        0x064E: _row(0x064E, "mark"),
        0xFC00: _row(0xFC00, "ayah_number"),
    }
    items = [
        Item(1, 1, 1, "word", BEH + FATHA + BEH, NBSP),
        Item(1, 1, 2, "ayah_end", "\ufc00", ""),
    ]
    records = spine.records(items, table)
    assert records[0] == {
        "id": "1:1:1",
        "kind": "word",
        "sep_after": "U+00A0",
        "clusters": [
            {"id": "1:1:1:1", "kind": "letter", "cps": "U+0628 U+064E"},
            {"id": "1:1:1:2", "kind": "letter", "cps": "U+0628"},
        ],
    }
    assert records[1]["clusters"] == [{"id": "1:1:2:1", "kind": "ayah_number", "cps": "U+FC00"}]
    assert spine.rebuild_ayahs(records) == {(1, 1): BEH + FATHA + BEH + NBSP + "\ufc00"}


def _row(cp: int, cls: str):
    from pipeline.codepoints import Row  # noqa: PLC0415

    return Row(cp, cls, "", "", "", "", "")


@needs_sources
def test_committed_spine_is_current_and_lossless() -> None:
    committed = spine.load_records()
    built = spine.build_records()
    assert committed == built
    rows = spine.source_rows()
    rebuilt = spine.rebuild_ayahs(committed)
    assert len(rebuilt) == len(rows)
    for row in rows:
        assert rebuilt[(int(row["sura_no"]), int(row["aya_no"]))] == row["aya_text"]


@needs_sources
def test_ids_are_unique_and_clusters_start_with_a_non_mark() -> None:
    table = load_table()
    records = spine.load_records()
    ids = [r["id"] for r in records] + [c["id"] for r in records for c in r["clusters"]]
    assert len(ids) == len(set(ids))
    for r in records:
        for c in r["clusters"]:
            first = int(c["cps"].split(" ")[0][2:], 16)
            assert table[first].cls != "mark", c["id"]


@needs_sources
def test_manifest_matches_files() -> None:
    manifest = spine.load_manifest()
    records = spine.load_records()
    assert manifest == spine.build_manifest(records)
    assert manifest["counts"]["items"] == dict(Counter(r["kind"] for r in records))
