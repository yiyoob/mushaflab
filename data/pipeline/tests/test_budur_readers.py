"""TB.2: reader-name resolution with the book's own terms (readers.yaml).

Sentences are synthetic (budur_sentences.yaml) and make no claim about any reading.
"""

from pipeline.budur import readers
from pipeline.tests.budur_fixtures import S

LEX = readers.load()
R = S["readers"]


def clause(name: str) -> readers.ClauseReaders:
    return LEX.resolve(readers.tokens(R[name]))


def test_name_key_unifies_alif_ya_and_drops_harakat() -> None:
    k = S["key"]
    assert readers.name_key(k["abi_harakat"]) == readers.name_key(k["abi_plain"])
    assert readers.name_key(k["ishaq_hamza"]) == readers.name_key(k["ishaq_plain"])
    assert readers.name_key(k["qalun_tatweel"]) == readers.name_key(k["qalun_plain"])


def test_qari_covers_both_ruwat_and_prefixes_are_stripped() -> None:
    c = clause("qari_and_prefixes")
    assert c.named == {"qalun", "warsh", "hafs", "shuba"}
    assert c.unresolved == []


def test_longest_form_wins() -> None:
    # "al-Duri 'an Abi 'Amr" is one form, not al-Duri + Abu 'Amr.
    assert clause("duri_of_abu_amr_long").named == {"duri_abuamr"}
    # "al-Basriyyan" (p. 11) is Abu 'Amr and Ya'qub; "al-Basri" alone is Abu 'Amr.
    assert clause("basriyyan").named == {"duri_abuamr", "susi", "ruways", "rawh"}
    assert clause("basri").named == {"duri_abuamr", "susi"}


def test_group_terms() -> None:
    assert clause("kufiyyun").named == {
        "hafs", "shuba", "khalaf_an_hamzah", "khallad", "abul_harith", "duri_kisai",
        "ishaq", "idris",
    }  # fmt: skip
    assert clause("madaniyyan").named == {"qalun", "warsh", "ibn_wardan", "ibn_jammaz"}


def test_khulf_attaches_to_the_named_reader() -> None:
    c = clause("khulf_last")
    assert c.named == {"bazzi", "qunbul", "qalun"}
    assert c.khulf == {"qalun"}
    assert clause("khulf_last_two").khulf == {"qalun", "duri_abuamr"}


def test_khulf_is_not_read_as_the_name_khalaf() -> None:
    c = clause("khulf_not_khalaf")
    assert c.named == {"warsh"}
    assert c.khulf == {"warsh"}


def test_unqualified_duri_follows_the_books_rule() -> None:
    # p. 11: after Abu 'Amr in the same statement it is al-Kisa'i's Duri ...
    assert clause("duri_after_basri").named == {"duri_abuamr", "susi", "duri_kisai"}
    # ... and "bi-khulf 'an al-Duri" is Abu 'Amr's Duri.
    assert clause("khulf_an_duri").khulf == {"duri_abuamr"}
    # Alone it is unresolved, never guessed.
    c = clause("duri_alone")
    assert c.named == set()
    assert c.unresolved == [R["duri_word"]]


def test_ambiguous_term_is_unresolved() -> None:
    c = clause("madani_alone")
    assert c.named == set()
    assert c.unresolved == [R["madani_word"]]


def test_complement_and_all_readers() -> None:
    assert clause("baqun").complement
    assert clause("ghayruhum").complement
    assert clause("all_readers").all_readers
    assert not clause("warsh_only").all_readers


def test_duri_al_kisai_by_his_name_ali_is_one_form() -> None:
    assert clause("duri_ali").named == {"duri_kisai"}


def test_every_ruwat_id_and_scope() -> None:
    assert len(LEX.ruwat) == 20
    assert set(LEX.in_scope) <= set(LEX.ruwat)
    assert len(LEX.in_scope) == 8
