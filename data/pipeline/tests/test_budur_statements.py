"""TB.3: blocks, clauses, scopes and sections of the book's prose (statements.py).

Sentences are synthetic (budur_sentences.yaml) and make no claim about any reading.
"""

from pipeline.budur import readers, statements
from pipeline.budur.epub import Para
from pipeline.tests.budur_fixtures import S

LEX = readers.load()
FEATS = statements.load_features()
W = S["words"]


def parse(name: str) -> list[statements.Block]:
    paras = [
        Para("page_1_0100", "1", 104, i, None, 1, "surahs", False, t)
        for i, t in enumerate(S["statements"][name], start=1)
    ]
    return statements.blocks(paras, LEX, FEATS)


def test_leading_quotes_open_a_block_and_continuations_join_it() -> None:
    bs = parse("two_blocks")
    assert [b.headwords for b in bs] == [[W["kalima"]], [W["lafz"], W["qawl"]]]
    assert bs[0].anchors == ["page_1_0100#p1", "page_1_0100#p2"]


def test_a_headword_inside_a_paragraph_after_a_waqf_term() -> None:
    bs = parse("inline_headword")
    assert [b.headwords for b in bs] == [[W["kalima"]], [W["lafz"]]]
    assert bs[1].kind == "backref"


def test_example_quotes_are_not_headwords() -> None:
    bs = parse("example_quote")
    assert [b.headwords for b in bs] == [[W["kalima"]]]
    assert bs[0].clauses[-1].examples == [W["lafz"]]


def test_complement_with_one_and_with_two_readings() -> None:
    (b,) = parse("simple_complement")
    comp = b.clauses[-1]
    assert comp.readers.complement
    assert comp.complement_in_scope == {
        "hafs", "shuba", "qalun", "bazzi", "qunbul", "duri_abuamr", "susi",
    }  # fmt: skip
    assert not comp.complement_ambiguous
    (b,) = parse("two_reading_complement")
    comp = b.clauses[-1]
    # After the clause just before it, or after every clause of the block: they differ.
    assert comp.complement_ambiguous
    assert comp.complement_in_scope is None


def test_feature_class_waqf_and_khulf() -> None:
    (b,) = parse("waqf_and_khulf")
    first, second = b.clauses
    assert (first.feature, first.wq) == ("phonetic", "waqf")
    assert (second.feature, second.readers.khulf) == ("textual", {"qalun"})


def test_scopes_and_exceptions() -> None:
    (b,) = parse("whole_quran_with_exception")
    (c,) = b.clauses
    assert c.scope == "whole_quran_word"
    assert c.exception_quotes == [W["lafz"]]
    assert parse("class_scope")[0].clauses[0].scope == "class"
    assert parse("surah_scope")[0].clauses[0].scope == "surah"


def test_obvious_and_count_qualifier() -> None:
    bs = parse("obvious_and_count")
    assert bs[0].kind == "obvious"
    assert (bs[1].count, bs[1].kind) == (2, "statement")


def test_rub_end_and_its_sections_and_the_next_rub() -> None:
    bs = parse("rub_sections")
    got = [(b.section, b.rub, b.headwords) for b in bs]
    assert got == [
        ("farsh", 0, [W["kalima"]]),
        ("rub_end", 0, [W["qawl"]]),
        ("mumal", 0, [W["kalima"]]),
        ("mumal", 0, [W["lafz"]]),
        ("mudgham_kabir", 0, [W["pair"]]),
        ("farsh", 1, [W["lafz"]]),
    ]
    mumal = bs[3].clauses[0]
    assert (mumal.feature, mumal.readers.named, mumal.readers.khulf) == (
        "phonetic", {"qalun"}, {"qalun"},
    )  # fmt: skip
    kabir = bs[4].clauses[0]
    assert kabir.readers.named == {"susi"}  # the book's default for al-kabir (p. 5)
    assert kabir.default_reader_cite == "page_1_0001 p. 5"


def test_all_readers_phrase() -> None:
    (b,) = parse("all_readers")
    assert b.clauses[0].readers.all_readers
