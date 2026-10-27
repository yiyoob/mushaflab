"""TB.5: splits, verdicts and attribution (compare.py), on synthetic units and forms.

Forms are placeholder strings ("a", "b"); nothing here is a claim about any reading.
"""

from pipeline.budur import compare
from pipeline.budur.readers import ClauseReaders
from pipeline.budur.statements import Block, Clause

R8 = ["hafs", "shuba", "warsh", "qalun", "bazzi", "qunbul", "duri_abuamr", "susi"]
IN = set(R8)


def cl(named: set[str] = frozenset(), feature: str | None = "textual", **kw: object) -> Clause:
    readers = ClauseReaders(
        named=set(named),
        khulf=set(kw.pop("khulf", set())),
        complement=bool(kw.pop("complement", False)),
        all_readers=bool(kw.pop("all_readers", False)),
    )
    c = Clause(
        text="",
        readers=readers,
        feature=feature,
        terms=[],
        wq=kw.pop("wq", None),
        scope=None,
        exception=False,
        exception_quotes=[],
        examples=[],
        awjuh=False,
        restriction=False,
        conditional=bool(kw.pop("conditional", False)),
    )
    c.complement_in_scope = kw.pop("complement_in_scope", None)
    c.complement_ambiguous = bool(kw.pop("complement_ambiguous", False))
    assert not kw
    return c


def block(*clauses: Clause, kind: str = "statement") -> Block:
    b = Block("b1", ["page_1_0011#p1"], "page_1_0011 p. 15", 1, "surahs", "farsh", 0, ["x"])
    b.clauses, b.kind = list(clauses), kind
    return b


def forms(**diff: str) -> dict[str, str]:
    return {r: diff.get(r, "a") for r in R8}


def no_diffs(f: dict[str, str]) -> dict[str, str]:
    return {r: ("" if f[r] == f["hafs"] else f"d{f[r]}") for r in R8}


def judge(u: compare.Unit, f: dict[str, str]) -> compare.Verdict:
    return compare.judge(u, f, no_diffs(f), R8)


def test_split_skips_waqf_conditional_and_phonetic_clauses() -> None:
    assert compare.clause_split(cl({"warsh"}), IN, "s").group == {"warsh"}
    assert compare.clause_split(cl({"warsh"}, wq="waqf"), IN, "s") is None
    assert compare.clause_split(cl({"warsh"}, conditional=True), IN, "s") is None
    assert compare.clause_split(cl({"warsh"}, feature="phonetic"), IN, "s") is None


def test_book_difference_present_in_data_is_no_finding() -> None:
    u = compare.entry_unit(block(cl({"warsh"})), [0], IN)
    v = judge(u, forms(warsh="b"))
    assert (v.findings, v.flagged) == ([], set())


def test_missing_difference_is_attributed_to_the_odd_one() -> None:
    u = compare.entry_unit(block(cl({"warsh"})), [0], IN)
    v = judge(u, forms())
    assert {f.kind for f in v.findings} == {"missing-difference"}
    assert v.flagged == {"warsh"}


def test_unstated_difference_on_hafs_flags_hafs() -> None:
    u = compare.entry_unit(block(cl({"warsh"})), [0], IN)
    v = judge(u, forms(warsh="b", hafs="c"))
    assert v.flagged == {"hafs"}


def test_khulf_reader_may_sit_on_either_side() -> None:
    u = compare.entry_unit(block(cl({"bazzi", "qunbul", "qalun"}, khulf={"qalun"})), [0], IN)
    assert judge(u, forms(bazzi="b", qunbul="b")).flagged == set()
    assert judge(u, forms(bazzi="b", qunbul="b", qalun="b")).flagged == set()


def test_the_one_that_fails_to_differ_is_flagged() -> None:
    u = compare.entry_unit(block(cl({"warsh", "qalun"})), [0], IN)
    assert judge(u, forms(warsh="b")).flagged == {"qalun"}


def test_tied_attribution_is_possible_not_flagged() -> None:
    u = compare.entry_unit(block(cl({"warsh", "qalun"})), [0], IN)
    v = judge(u, forms(warsh="b", qalun="c"))  # only warsh-qalun disagree: either may be off
    assert v.flagged == set()
    assert v.possible == {"warsh", "qalun"}


def test_phonetic_statement_explains_a_data_difference() -> None:
    u = compare.entry_unit(block(cl({"susi"}, feature="phonetic")), [0], IN)
    v = judge(u, forms(susi="b"))
    assert v.findings == []
    assert [why for _, why in v.explained] == ["phonetic"] * 7


def test_maghribi_family_pattern_is_a_convention_candidate() -> None:
    u = compare.entry_unit(block(cl({"bazzi"})), [0], IN)
    f = forms(bazzi="b", warsh="m", qalun="m")
    v = compare.judge(u, f, {**no_diffs(f), "warsh": "dm", "qalun": "dm"}, R8)
    assert v.flagged == set()
    assert {why for _, why in v.explained} == {"convention-family"}


def test_silence_unit_flags_patch_without_statement() -> None:
    u = compare.Unit("w0", "silence", [0], set())
    v = judge(u, forms(susi="b"))
    assert {f.kind for f in v.findings} == {"patch-without-statement"}
    assert v.flagged == {"susi"}


def test_coverage_is_implicit_once_one_of_the_8_is_covered() -> None:
    u = compare.entry_unit(block(cl({"warsh"})), [0], IN)
    assert (u.explicit, u.covered) == ({"warsh"}, IN)
    other = compare.entry_unit(block(cl(set())), [0], IN)  # e.g. only Hamzah named
    assert other.covered == set()


def test_ambiguous_complement_has_two_readings() -> None:
    b = block(
        cl({"bazzi", "qunbul"}),
        cl({"khallad"}),  # khallad: outside the 8
        cl(set(), complement=True, complement_ambiguous=True),
    )
    assert compare.ambiguous_complement(b)
    prev = compare.entry_unit(b, [0], IN, "prev")
    every = compare.entry_unit(b, [0], IN, "all")
    assert len(prev.splits) == 3
    assert prev.splits[-1].group == IN  # after a clause naming none of the 8
    assert every.splits[-1].group == IN - {"bazzi", "qunbul"}


def test_min_covers() -> None:
    assert compare.min_covers([], R8) == [frozenset()]
    assert compare.min_covers([("hafs", "warsh"), ("hafs", "qalun")], R8) == [frozenset({"hafs"})]
