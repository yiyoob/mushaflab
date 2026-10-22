"""T4.3: word alignment of a riwayah text on the Hafs spine.

Synthetic cases use Latin letters and an identity key, so they test the alignment logic only;
they are not Quranic text. Integration tests read the pinned sources.
"""

import pytest

from pipeline import align, ingest_riwayah, kfgqpc
from pipeline.align import Pair, align_surah
from pipeline.ingest_riwayah import Item
from pipeline.tests.markers import needs_sources


def words(spec: str, ayah: int = 1) -> list[Item]:
    """`"ab cd @"` -> word items and an ayah end; `#` is a hizb sign."""
    out = []
    pos = 0
    for tok in spec.split():
        if tok == "@":
            out.append(Item(1, ayah, pos + 1, "ayah_end", tok, ""))
            ayah += 1
            pos = 0
        elif tok == "#":
            out.append(Item(1, ayah, pos, "hizb", tok, " "))
        else:
            pos += 1
            out.append(Item(1, ayah, pos, "word", tok, " "))
    return out


def key(i: Item) -> str:
    return i.text


def shape(pairs: list[Pair]) -> list[tuple[str | None, str | None, str]]:
    return [(p.h.text if p.h else None, p.r.text if p.r else None, p.how) for p in pairs]


def test_identical_texts_pair_everything_as_equal() -> None:
    h = words("ab cd @ ef #  gh @")
    pairs = align_surah(h, words("ab cd @ ef #  gh @"), key)
    assert all(p.how == "equal" for p in pairs)
    assert [(p.h, p.r) for p in pairs] == list(zip(h, words("ab cd @ ef # gh @"), strict=True))


def test_same_length_replacement_pairs_by_position() -> None:
    pairs = align_surah(words("ab cd ef @"), words("ab xy ef @"), key)
    assert shape(pairs) == [
        ("ab", "ab", "equal"),
        ("cd", "xy", "position"),
        ("ef", "ef", "equal"),
        ("@", "@", "equal"),
    ]


def test_inserted_and_deleted_words_stay_unpaired() -> None:
    pairs = align_surah(words("ab cd ef @"), words("ab ef zz @"), key)
    assert shape(pairs) == [
        ("ab", "ab", "equal"),
        ("cd", None, "unpaired"),
        ("ef", "ef", "equal"),
        (None, "zz", "unpaired"),
        ("@", "@", "equal"),
    ]


def test_unequal_blocks_pair_by_edit_distance() -> None:
    # one Hafs word against two riwayah words: the closer one is paired
    pairs = align_surah(words("aa bcdef gg @"), words("aa q bcdxf gg @"), key)
    assert shape(pairs) == [
        ("aa", "aa", "equal"),
        (None, "q", "unpaired"),
        ("bcdef", "bcdxf", "fuzzy"),
        ("gg", "gg", "equal"),
        ("@", "@", "equal"),
    ]


def test_kinds_never_pair_across() -> None:
    # an ayah end moved by one word: ayah ends pair only with ayah ends
    pairs = align_surah(words("ab @ cd ef @"), words("ab cd @ ef @"), key)
    assert all(p.h is None or p.r is None or p.h.kind == p.r.kind for p in pairs)
    hs = [p.h for p in pairs if p.h]
    rs = [p.r for p in pairs if p.r]
    assert hs == words("ab @ cd ef @")
    assert rs == words("ab cd @ ef @")


def test_alignment_is_deterministic_and_order_preserving() -> None:
    h, r = words("a b c d e f @"), words("a c b d f e @")
    first = align_surah(h, r, key)
    assert first == align_surah(h, r, key)
    assert [p.h for p in first if p.h] == h
    assert [p.r for p in first if p.r] == r


@needs_sources
@pytest.mark.parametrize("riwayah", kfgqpc.RIWAYAT[1:])
def test_real_alignment_keeps_both_texts_complete_and_in_order(riwayah: str) -> None:
    hafs = ingest_riwayah.load("hafs").items
    text = ingest_riwayah.load(riwayah).items
    pairs = align.align_text(hafs, text)
    assert [p.h for p in pairs if p.h] == hafs
    assert [p.r for p in pairs if p.r] == text
    assert all(p.h is None or p.r is None or p.h.kind == p.r.kind for p in pairs)
    assert all(p.h is None or p.r is None or p.h.surah == p.r.surah for p in pairs)


@needs_sources
def test_hafs_against_itself_is_all_equal() -> None:
    hafs = ingest_riwayah.load("hafs").items
    assert {p.how for p in align.align_text(hafs, hafs)} == {"equal"}
