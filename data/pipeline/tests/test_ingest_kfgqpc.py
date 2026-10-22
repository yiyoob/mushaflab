"""T1.1: ingest of the KFGQPC Hafs text into spine items.

Synthetic cases use arbitrary letters (U+0628, U+062A) to exercise structure only; they are
not Quranic text. Integration tests read the pinned source and are skipped when it has
not been fetched (CI always fetches it).
"""

from collections import Counter
from itertools import pairwise

import pytest

from pipeline import sources
from pipeline.ingest_kfgqpc import (
    IngestError,
    decode_ayah_sign,
    ingest,
    load_rows,
    parse_ayah,
    rebuild_ayah,
)
from pipeline.tests.markers import needs_sources

BEH, TEH, FATHA = "\u0628", "\u062a", "\u064e"
NBSP, HIZB, SAJDAH = "\u00a0", "\u06de", "\u06e9"


def sign(n: int) -> str:
    return chr(0xFC00 + n - 1)


# --- decode_ayah_sign ---------------------------------------------------------------------


def test_decode_ayah_sign_first_and_last_used_values() -> None:
    assert decode_ayah_sign("\ufc00") == 1
    assert decode_ayah_sign("\ufd1d") == 286


@pytest.mark.parametrize("bad", ["", "\ufc00\ufc01", "\u0661", "a", "\ufbff"])
def test_decode_ayah_sign_rejects_non_signs(bad: str) -> None:
    with pytest.raises(IngestError):
        decode_ayah_sign(bad)


# --- parse_ayah (synthetic) ---------------------------------------------------------------


def test_parse_simple_ayah() -> None:
    text = f"{BEH}{FATHA} {TEH}{NBSP}{sign(1)}"
    items = parse_ayah(1, 1, text)
    assert [(i.id, i.kind, i.sep_after) for i in items] == [
        ("1:1:1", "word", " "),
        ("1:1:2", "word", NBSP),
        ("1:1:3", "ayah_end", ""),
    ]
    assert items[0].text == BEH + FATHA
    assert rebuild_ayah(items) == text


def test_parse_hizb_gets_position_zero() -> None:
    text = f"{HIZB}{NBSP}{BEH} {TEH}{NBSP}{sign(2)}"
    items = parse_ayah(3, 2, text)
    assert [(i.id, i.kind) for i in items] == [
        ("3:2:0", "hizb"),
        ("3:2:1", "word"),
        ("3:2:2", "word"),
        ("3:2:3", "ayah_end"),
    ]
    assert rebuild_ayah(items) == text


def test_sajdah_stays_inside_its_word() -> None:
    text = f"{BEH} {TEH}{SAJDAH}{NBSP}{sign(5)}"
    items = parse_ayah(9, 5, text)
    assert items[1].text == TEH + SAJDAH
    assert rebuild_ayah(items) == text


def test_regular_space_before_sign_is_preserved() -> None:
    text = f"{BEH} {sign(4)}"
    items = parse_ayah(2, 4, text)
    assert items[0].sep_after == " "
    assert rebuild_ayah(items) == text


@pytest.mark.parametrize(
    ("text", "why"),
    [
        (f"{BEH}{NBSP}{sign(2)}", "does not match"),  # sign says 2, ayah is 1
        (f"{BEH} {HIZB}{NBSP}{sign(1)}", "hizb"),  # hizb not first
        (f"{BEH}  {TEH}{NBSP}{sign(1)}", "separator"),  # double separator
        (f" {BEH}{NBSP}{sign(1)}", "separator"),  # leading separator
        (f"{sign(1)} {BEH}", "last"),  # sign not last
        (f"{BEH} {TEH}", "no ayah sign"),  # missing sign
    ],
)
def test_parse_rejects_unexpected_structure(text: str, why: str) -> None:
    with pytest.raises(IngestError, match=why):
        parse_ayah(1, 1, text)


# --- integration with the pinned source ---------------------------------------------------


# Computed from kfgqpc-official-hafs (UthmanicHafs_v2-0.zip, sha256 a7b0e5591945712e...,
# member hafsData_v2-0.json) during Phase 1 exploration; see the Phase 1 plan.
EXPECTED_COUNTS = {"word": 77430, "ayah_end": 6236, "hizb": 199}


@needs_sources
def test_every_ayah_rebuilds_exactly() -> None:
    rows = load_rows()
    by_ayah: dict[tuple[int, int], list] = {}
    for item in ingest(rows):
        by_ayah.setdefault((item.surah, item.ayah), []).append(item)
    assert len(by_ayah) == len(rows)
    for row in rows:
        key = (int(row["sura_no"]), int(row["aya_no"]))
        assert rebuild_ayah(by_ayah[key]) == row["aya_text"], key


@needs_sources
def test_item_counts_match_source_constants() -> None:
    counts = Counter(i.kind for i in ingest(load_rows()))
    assert dict(counts) == EXPECTED_COUNTS


@needs_sources
def test_ayah_counts_agree_with_two_independent_sources() -> None:
    kfgqpc = Counter(int(r["sura_no"]) for r in load_rows())
    qul = sources.qul_ayah_counts_per_surah()
    tanzil = sources.tanzil_ayah_counts_per_surah()
    assert kfgqpc == qul == tanzil
    assert len(kfgqpc) == max(kfgqpc)  # surahs are numbered 1..n with no gap


@needs_sources
def test_separator_before_ayah_sign_is_preserved_including_2_286() -> None:
    # Source anomaly (G-025): 2:286 is the only ayah with U+0020 before its ayah sign.
    items = ingest(load_rows())
    before_sign = {
        (prev.surah, prev.ayah): prev.sep_after
        for prev, nxt in pairwise(items)
        if nxt.kind == "ayah_end"
    }
    odd = {k: v for k, v in before_sign.items() if v != NBSP}
    assert odd == {(2, 286): " "}


@needs_sources
def test_2_6_leading_separator_anomaly_is_kept_exactly() -> None:
    # Source anomaly (G-026): the only aya_text that starts with a separator (U+0020).
    items = ingest(load_rows())
    leading = {(i.surah, i.ayah): i.sep_before for i in items if i.sep_before}
    assert leading == {(2, 6): " "}


def test_leading_separator_allowed_only_for_listed_anomalies() -> None:
    text = f" {BEH}{NBSP}{sign(6)}"
    items = parse_ayah(2, 6, text)
    assert items[0].sep_before == " "
    assert rebuild_ayah(items) == text
    with pytest.raises(IngestError, match="separator"):
        parse_ayah(2, 7, f" {BEH}{NBSP}{sign(7)}")
