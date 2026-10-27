"""TB.5: our word forms from raw patches and the loose projection (forms.py).

Synthetic words and patches; codepoints only, no claim about any reading.
"""

from pipeline.budur import forms
from pipeline.codepoints import load_table
from pipeline.spine import from_cps

T = load_table()
W = forms.Word(
    "1:1:1",
    1,
    1,
    0,
    (("1:1:1:1", from_cps("U+0643 U+064E")), ("1:1:1:2", from_cps("U+062A U+06E1"))),
)


def patches(*ps: dict) -> forms.Patches:
    out = forms.Patches("x")
    for p in ps:
        out.by_word.setdefault("1:1:1", []).append(p)
    return out


def test_patches_apply_like_the_resolver() -> None:
    p = patches(
        {
            "id": "a",
            "op": "set_marks",
            "target": "1:1:1:2",
            "payload": {"remove": ["U+06E1"], "add": ["U+0652"]},
        },
        {
            "id": "b",
            "op": "replace",
            "target": "1:1:1:1",
            "payload": {"kind": "letter", "cps": "U+0642 U+064F"},
        },
    )
    assert forms.our_text(W, p) == from_cps("U+0642 U+064F U+062A U+0652")


def test_deleted_word_has_no_form() -> None:
    assert forms.our_text(W, patches({"id": "d", "op": "delete", "target": "1:1:1"})) is None


def test_confirmed_encoding_rows_map_back_to_hafs() -> None:
    p = patches(
        {
            "id": "a",
            "op": "set_marks",
            "target": "1:1:1:2",
            "payload": {"remove": ["U+06E1"], "add": ["U+0652"]},
        }
    )
    p.to_hafs = {from_cps("U+0652"): from_cps("U+06E1")}
    assert forms.our_text(W, p) == W.text


def conv(**kw: object) -> forms.Conventions:
    return forms.Conventions(
        drop=frozenset(kw.get("drop", ())), merge=dict(kw.get("merge", {})),
        cluster_rules=dict(kw.get("cluster_rules", {})),
        word_level=frozenset(kw.get("word_level", ())), sort_marks=True, rows=[],
    )  # fmt: skip


def test_loose_drops_merges_and_moves_word_level_marks() -> None:
    sukun, khah = from_cps("U+0652"), from_cps("U+06E1")
    c = conv(merge={khah: sukun}, drop={from_cps("U+06D6")})
    assert forms.loose(from_cps("U+062A U+06E1 U+06D6"), c, T) == from_cps("U+062A U+0652")
    c = conv(word_level={from_cps("U+0657")})
    a = forms.loose(from_cps("U+062F U+0657 U+0649"), c, T)
    b = forms.loose(from_cps("U+062F U+0649 U+0657"), c, T)
    assert a == b


def test_loose_cluster_rule_drops_marks_on_its_base() -> None:
    alef = from_cps("U+0627")
    c = conv(cluster_rules={alef: frozenset(from_cps("U+0650 U+06EC"))})
    assert forms.loose(from_cps("U+0627 U+0650 U+06EC U+0644"), c, T) == from_cps("U+0627 U+0644")


def test_conventions_file_rows_are_all_hypotheses_with_evidence() -> None:
    c = forms.load_conventions()
    assert c.rows
    assert all(r["evidence"] for r in c.rows)
