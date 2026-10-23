"""T5.1: riwayah profiles.

Synthetic cases use made-up pages, not Quranic data. Integration tests check the profiles
against the pinned KFGQPC rows and the tracked 1441H layout, never against expected values
about the Quran (rule 1).
"""

import pytest

from pipeline import ingest_layout, kfgqpc, profiles
from pipeline.tests.markers import needs_sources

LINES = {1: 8, 2: 15, 3: 15}


def row(sura: int, aya: int, page: str, start: int, end: int) -> dict:
    return {"sura_no": sura, "aya_no": aya, "page": page, "line_start": start, "line_end": end}


def test_gap_lines_before_each_surah() -> None:
    rows = [
        row(1, 1, "1", 2, 3),
        row(1, 2, "1", 4, 8),
        row(2, 1, "2", 3, 4),
        row(2, 2, "2", 5, 13),
        row(3, 1, "3", 1, 2),  # header lines at the end of the previous page
    ]
    assert profiles.gap_lines(rows, LINES) == {
        1: [(1, 1)],
        2: [(2, 1), (2, 2)],
        3: [(2, 14), (2, 15)],
    }


def test_gap_after_a_page_crossing_ayah_starts_on_its_last_page() -> None:
    rows = [row(1, 1, "1", 2, 8), row(1, 2, "2-3", 1, 1), row(2, 1, "3", 4, 5)]
    assert profiles.gap_lines(rows, LINES) == {1: [(1, 1)], 2: [(3, 2), (3, 3)]}


def test_header_kinds_come_from_the_hafs_layout_or_are_hypotheses() -> None:
    hafs = {(1, 1): "surah_name", (2, 1): "surah_name", (2, 2): "basmalah", (1, 2): "text"}
    gaps = {1: [(1, 1), (1, 2)], 2: [(2, 1), (2, 2)]}
    headers = profiles.headers(gaps, hafs)
    assert headers == [
        {"surah": 1, "page": 1, "name_line": 1, "basmalah_line": 2,
         "basmalah_evidence": profiles.G023},
        {"surah": 2, "page": 2, "name_line": 1, "basmalah_line": 2},
    ]  # fmt: skip


def test_unexplained_gap_lines_are_an_error() -> None:
    with pytest.raises(profiles.ProfileError):
        profiles.headers({5: [(9, 3), (9, 4), (9, 5)]}, {})
    with pytest.raises(profiles.ProfileError):  # the first gap line is not a surah name
        profiles.headers({5: [(9, 3)]}, {(9, 3): "basmalah"})


@needs_sources
def test_hafs_headers_equal_the_layout_header_lines() -> None:
    layout = ingest_layout.load_layout()
    expected = {(ln["page"], ln["line"]) for ln in layout["lines"] if ln["type"] != "text"}
    got = set()
    for h in profiles.build("hafs")["surah_headers"]:
        got.add((h["page"], h["name_line"]))
        if "basmalah_line" in h:
            got.add((h["page"], h["basmalah_line"]))
    assert got == expected


@needs_sources
@pytest.mark.parametrize("riwayah", kfgqpc.RIWAYAT)
def test_profile_names_its_sources(riwayah: str) -> None:
    p = profiles.build(riwayah)
    assert p["riwayah"] == riwayah
    assert p["text_source"] == kfgqpc.source_id(riwayah)
    assert len(p["surah_headers"]) == 114
    if riwayah == "hafs":
        assert p["patch_stack"] == []
        assert "ayah_map" not in p
    else:
        assert p["patch_stack"] == [f"patches/raw/{riwayah}.yaml"]
        assert p["ayah_map"] == riwayah
        assert p["single_source"] is True


@needs_sources
def test_tracked_profiles_are_current() -> None:
    assert profiles.main(["", "check"]) == 0
