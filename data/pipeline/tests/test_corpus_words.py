"""M0.4: the word table holds the Hafs spine exactly, item for item."""

import json

from pipeline import corpus_words, spine
from pipeline.clusterize import clusterize
from pipeline.codepoints import load_table


def _rows() -> list[list[str]]:
    lines = corpus_words.OUT.read_text(encoding="utf-8").split("\n")
    body = [ln for ln in lines if ln and not ln.startswith("#")]
    assert body[0].split("\t") == corpus_words.HEADER
    return [ln.split("\t") for ln in body[1:]]


def test_tracked_table_is_current() -> None:
    assert corpus_words.OUT.read_text(encoding="utf-8") == corpus_words.render()


def test_every_spine_item_round_trips() -> None:
    table = load_table()
    items = corpus_words.spine_items()
    rows = _rows()
    assert len(rows) == len(items)
    for row, it in zip(rows, items, strict=True):
        word_id, kind, text, sep_before, sep_after, _source = row
        assert (word_id, kind, sep_before, sep_after) == (
            it["id"],
            it["kind"],
            it.get("sep_before", ""),
            it["sep_after"],
        )
        got = [(c.kind, spine.to_cps(c.text)) for c in clusterize(text, table)]
        want = [(c["kind"], c["cps"]) for c in it["clusters"]]
        assert got == want, word_id


def test_counts_match_the_spine_manifest() -> None:
    manifest = json.loads(spine.MANIFEST.read_text(encoding="utf-8"))
    kinds: dict[str, int] = {}
    for row in _rows():
        kinds[row[1]] = kinds.get(row[1], 0) + 1
    assert kinds == manifest["counts"]["items"]
