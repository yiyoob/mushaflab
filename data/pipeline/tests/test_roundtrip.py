"""T4.6: the round-trip comparison (D22) and the version-drift grouping.

Synthetic texts use Latin letters; they test the comparison logic only.
"""

from pipeline import drift, roundtrip
from pipeline.roundtrip import RItem, check, split_ayahs


def rows(*texts: str) -> list[dict]:
    return [{"sura_no": 1, "aya_no": n, "aya_text": t} for n, t in enumerate(texts, start=1)]


ITEMS = [
    RItem("1:1:1", "word", "ab "),
    RItem("1:1:2", "ayah_end", "1"),
    RItem("1:2:0", "hizb", "# "),
    RItem("1:2:1+ins.1", "word", "cd "),
    RItem("1:2:2", "ayah_end", "2"),
]


def test_exact_text_passes() -> None:
    res = check("x", ITEMS, rows("ab 1", "# cd 2"), None)
    assert res.ok
    assert (res.ayahs, res.exact) == (2, 2)


def test_split_after_each_ayah_end() -> None:
    assert [[i.id for i in g] for g in split_ayahs(ITEMS)] == [
        ["1:1:1", "1:1:2"],
        ["1:2:0", "1:2:1+ins.1", "1:2:2"],
    ]


def test_one_codepoint_off_fails_and_is_located() -> None:
    res = check("x", ITEMS, rows("ab 1", "# ce 2"), None)
    assert not res.ok
    assert res.exact == 1
    assert any("1:2" in p and "U+0064" in p for p in res.problems)


def test_same_text_but_moved_ayah_boundary_fails() -> None:
    res = check("x", ITEMS, rows("ab 1# ", "cd 2"), None)
    assert not res.ok


def test_ayah_map_must_name_each_ayahs_first_and_last_item() -> None:
    good = {
        "entries": [
            {"surah": 1, "ayah": 1, "first": "1:1:1", "last": "1:1:2"},
            {"surah": 1, "ayah": 2, "first": "1:2:0", "last": "1:2:2"},
        ]
    }
    assert check("x", ITEMS, rows("ab 1", "# cd 2"), good).ok
    bad = {"entries": [dict(e) for e in good["entries"]]}
    bad["entries"][1]["first"] = "1:2:1+ins.1"
    assert not check("x", ITEMS, rows("ab 1", "# cd 2"), bad).ok
    assert not check("x", ITEMS, rows("ab 1", "# cd 2"), {"entries": good["entries"][:1]}).ok


def test_drift_groups_identical_changes() -> None:
    a = {(1, 1): "abc", (1, 2): "xbc", (1, 3): "q", (2, 1): "z"}
    b = {(1, 1): "ac", (1, 2): "xc", (1, 3): "q", (3, 1): "z"}
    result = drift.compare(a, b)
    assert result["differing"] == 2
    assert result["grouped"] == {("U+0062 -> nothing",): [(1, 1), (1, 2)]}
    assert (result["only_a"], result["only_b"]) == ([(2, 1)], [(3, 1)])


def test_words_without_number_drops_separators_and_the_ayah_number() -> None:
    sign = "\ufc00"
    assert drift.words_without_number(f"ab cd\u00a0{sign}") == "ab cd"
    assert drift.words_without_number(f"ab  cd{sign}") == "ab cd"
    assert roundtrip.SHOWN > 0
