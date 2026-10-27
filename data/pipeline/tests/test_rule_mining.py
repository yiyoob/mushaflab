"""T6.4: rule induction on a synthetic spine (Latin letters only, rule 1)."""

from collections import defaultdict

from pipeline import rule_mining as rm
from pipeline.spine import to_cps


def view(words: list[str]) -> rm.View:
    """One ayah of words; each letter is one cluster."""
    items, texts, pos, by_first = [], [], {}, defaultdict(list)
    for i, w in enumerate(words):
        clusters = [
            {"id": f"1:1:{i + 1}:{k + 1}", "cps": to_cps(c), "kind": "letter"}
            for k, c in enumerate(w)
        ]
        items.append({"id": f"1:1:{i + 1}", "kind": "word", "clusters": clusters})
        texts.append(list(w))
        for k, c in enumerate(w):
            pos[clusters[k]["id"]] = (i, k)
            by_first[c].append((i, k))
    return rm.View(items, texts, pos, dict(by_first))


def patch(n: int, target: str, cps: str) -> dict:
    return {
        "id": f"p{n}",
        "op": "replace",
        "target": target,
        "payload": {"kind": "letter", "cps": cps},
    }


def test_matcher_is_leftmost_and_non_overlapping() -> None:
    v = view(["aab", "ab"])
    pat = rm.pattern_for("a", ("next", "a"))
    assert rm.match_set(pat, v) == ["1:1:1:1"]  # "a a" once; the second a was consumed
    assert rm.match_set(rm.pattern_for("a", ("next", "b")), v) == ["1:1:1:2", "1:1:2:1"]
    assert rm.match_set(rm.pattern_for("b", ("at", "last")), v) == ["1:1:1:3", "1:1:2:2"]
    nw = rm.pattern_for("b", ("nextword", "a"))
    assert rm.match_set(nw, v) == ["1:1:1:3"]


def test_mining_finds_the_context_and_lists_exceptions() -> None:
    # "x" becomes "y" before "b" in four words; a fifth "xb" is left unchanged (exception
    # allowed: 1 of 5 matches is within MAX_EXCEPTIONS); "xc" never changes.
    v = view(["xb", "xb", "xb", "xb", "xb", "xc"])
    raw = [patch(i, f"1:1:{i}:1", "U+0079") for i in range(1, 5)]
    rules, explained = rm.mine("t", raw, v)
    assert len(rules) == 1
    m = rules[0]
    assert m.feature == ("next", "b")
    assert m.hits == ["1:1:1:1", "1:1:2:1", "1:1:3:1", "1:1:4:1"]
    assert m.exceptions == [("1:1:5:1", "KFGQPC t text: no such change here")]
    assert explained == {"p1", "p2", "p3", "p4"}


def test_too_many_exceptions_make_no_rule() -> None:
    v = view(["x"] * 10)
    raw = [patch(i, f"1:1:{i}:1", "U+0079") for i in range(1, 4)]
    rules, explained = rm.mine("t", raw, v)
    assert rules == [] and explained == set()


def test_residual_drops_exactly_the_explained_patches() -> None:
    text = 'spine: "hafs/v1"\npatches:\n  - {id: "p1", x: 1}\n  - {id: "p2", x: 2}\n'
    out = rm.residual_file("t", text, {"p1"})
    assert '"p1"' not in out and '"p2"' in out and "patches:" in out
