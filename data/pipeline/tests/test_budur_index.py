"""TB.3: the rule index and the corrections appendix (index.py), on synthetic paragraphs."""

from pipeline.budur import index, readers, statements
from pipeline.budur.epub import Para
from pipeline.tests.budur_fixtures import S

LEX = readers.load()
FEATS = statements.load_features()
X = S["index"]


def para(page: str, i: int, text: str, zone: str = "surahs", printed: int = 104) -> Para:
    return Para(page, "1", printed, i, None, 1 if zone == "surahs" else None, zone, False, text)


def test_only_scoped_clauses_become_rules() -> None:
    bs = statements.blocks(
        [para("page_1_0100", 1, X["scoped"]), para("page_1_0100", 2, X["plain"])], LEX, FEATS
    )
    (r,) = index.rules(bs, set(LEX.in_scope))
    assert (r.scope, r.readers, r.applied, r.cite) == (
        "whole_quran_word", {"warsh"}, True, "page_1_0100 p. 104",
    )  # fmt: skip


def test_corrections_link_to_the_printed_page_they_name() -> None:
    bs = statements.blocks(
        [
            para("page_1_0100", 1, X["plain"]),
            para("page_2_0355", 1, X["correction"], "corrections", 2),
            para("page_2_0355", 2, X["correction_more"], "corrections", 2),
            para("page_2_0361", 1, X["mudakhala"], "corrections", 8),
        ],
        LEX, FEATS,
    )  # fmt: skip
    c1, c2 = index.corrections(bs)
    assert (c1.number, c1.page, c1.blocks) == (3, 104, [bs[0].id])
    assert X["correction_more"] in c1.text
    assert (c2.number, c2.page, c2.blocks) == (1, None, [])
