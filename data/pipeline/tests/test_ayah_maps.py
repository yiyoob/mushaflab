"""T4.2: ayah maps and numbering findings.

Synthetic cases use Latin letters (not Quranic text). Integration tests check the maps
against the pinned sources and the counts against each other, never against expected
values about the Quran (rule 1).
"""

import pytest

from pipeline import ayah_maps, emit_patches, kfgqpc
from pipeline.align import Pair
from pipeline.differ import Change
from pipeline.ingest_riwayah import Item
from pipeline.tests.markers import needs_sources


def test_resolved_ids_use_spine_ids_or_insert_keys() -> None:
    h = Item(1, 1, 1, "word", "ab", " ")
    r1 = Item(1, 1, 1, "word", "ab", " ")
    r2 = Item(1, 1, 2, "word", "cd", " ")
    pairs = [Pair(h, r1, "equal"), Pair(None, r2, "unpaired")]
    ins = Change("1:1:1", "insert_after", {"seq": 1}, "item", "item-insert", "word", "", "cd",
                 "1:1:2", 1)  # fmt: skip
    assert ayah_maps.resolved_ids(pairs, [ins]) == {"1:1:1": "1:1:1", "1:1:2": "1:1:1+ins.1"}


@needs_sources
@pytest.mark.parametrize("riwayah", emit_patches.ORDER)
def test_map_lists_every_ayah_once_in_order(riwayah: str) -> None:
    entries = ayah_maps.entries(riwayah)
    rows = kfgqpc.rows(riwayah)
    assert [(e["surah"], e["ayah"]) for e in entries] == [
        (int(r["sura_no"]), int(r["aya_no"])) for r in rows
    ]


@needs_sources
@pytest.mark.parametrize("riwayah", emit_patches.ORDER)
def test_boundary_findings_account_for_the_ayah_count(riwayah: str) -> None:
    f = ayah_maps.findings(riwayah)
    hafs_ayahs = len(kfgqpc.rows("hafs"))
    lost = sum(1 for side, _ in f["unpaired"] if side == "Hafs")
    gained = len(f["unpaired"]) - lost
    assert f["ayahs"] == hafs_ayahs - len(f["only_hafs"]) + len(f["only_riw"]) - lost + gained


@needs_sources
def test_tracked_maps_are_current() -> None:
    assert ayah_maps.main(["", "check"]) == 0
