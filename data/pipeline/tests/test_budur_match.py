"""TB.4: letter keys and order-constrained placement (match.py), on a synthetic spine.

The spine words below are placeholder letter strings, not Quranic text; the tests check the
key and alignment logic only.
"""

from pipeline.budur import match
from pipeline.budur.forms import Word
from pipeline.codepoints import load_table
from pipeline.tests.budur_fixtures import S

T = load_table()
M = S["match"]


def spine(texts: list[str]) -> list[Word]:
    return [Word(f"1:1:{i + 1}", 1, 1, i, ((f"1:1:{i + 1}:1", t),)) for i, t in enumerate(texts)]


def test_book_and_spine_keys_meet() -> None:
    for book, sp in M["same_key"]:
        assert match.book_key(book) == match.spine_key(sp, T), (book, sp)


def test_tiers() -> None:
    idx = match.Index(spine(M["tier_spine"]), T)
    got = [(c.start, c.tier) for c in idx.candidates(M["tier_quote"], 0, 2)]
    assert got == [(0, "exact"), (1, "prefix")]


def test_multi_word_quote_and_word_prefix() -> None:
    idx = match.Index(spine(M["multi_spine"]), T)
    (c,) = idx.candidates(M["multi_quote"], 0, len(M["multi_spine"]) - 1)
    assert (c.spans, c.tier) == (((1, 2),), "prefix")


def cand(*starts: int) -> list[match.Candidate]:
    return [match.Candidate(((s, s),), "exact") for s in starts]


def test_alignment_uses_order_to_pick_among_candidates() -> None:
    # Entry 0 is unique at 5; entry 1 occurs at 2 and 8: only 8 comes after 5.
    got = match.align([cand(5), cand(2, 8), cand(9)], 10)
    assert [(p.status, [c.start for c in p.places]) for p in got] == [
        ("matched", [5]), ("matched", [8]), ("matched", [9]),
    ]  # fmt: skip


def test_two_equal_chains_leave_the_entry_ambiguous() -> None:
    got = match.align([cand(1), cand(3, 4), cand(6)], 10)
    assert got[1].status == "ambiguous"


def test_unique_candidate_out_of_book_order_is_matched_and_marked() -> None:
    # 1-7-8 and 1-4-8 are both longest: 7 and 4 conflict, and order cannot say which moved.
    got = match.align([cand(1), cand(7), cand(4), cand(8)], 10)
    assert [p.status for p in got] == ["matched"] * 4
    assert [p.out_of_order for p in got] == [False, True, True, False]


def test_no_candidate_is_unmatched() -> None:
    assert match.align([cand(1), []], 10)[1].status == "unmatched"


def test_combine_for_entries_naming_two_places() -> None:
    (c,) = match.combine(cand(2, 6), 2)
    assert c.spans == ((2, 2), (6, 6))


def test_list_entries_need_exactly_their_count() -> None:
    assert match.place_list(cand(3), None).status == "matched"
    assert match.place_list(cand(3, 4), None).status == "ambiguous"
    assert match.place_list(cand(3, 4), 2).status == "matched"
    assert match.place_list([], None).status == "unmatched"
