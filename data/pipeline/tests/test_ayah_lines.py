"""T5.2: per-ayah page and line constraints, copied from the pinned KFGQPC files."""

import pytest

from pipeline import ayah_lines, kfgqpc
from pipeline.tests.markers import needs_sources


@needs_sources
@pytest.mark.parametrize("riwayah", kfgqpc.RIWAYAT)
def test_one_entry_per_ayah_with_its_page_range(riwayah: str) -> None:
    rows = kfgqpc.rows(riwayah)
    entries = ayah_lines.entries(riwayah)
    assert len(entries) == len(rows)
    for e, r in zip(entries, rows, strict=True):
        assert (e["surah"], e["ayah"]) == (int(r["sura_no"]), int(r["aya_no"]))
        assert e["last_page"] in (e["page"], e["page"] + 1)
        assert str(r["page"]) in (str(e["page"]), f"{e['page']}-{e['last_page']}")


@needs_sources
def test_tracked_files_are_current() -> None:
    assert ayah_lines.main(["", "check"]) == 0
