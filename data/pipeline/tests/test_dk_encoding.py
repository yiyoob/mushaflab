"""T3.1: the KFGQPC -> DigitalKhatt Madina encoding table is derived from data (D2, D14).

Unit tests use synthetic Latin "clusters" (test fixtures, not Quranic text). Tests marked
needs_sources check the real table against the pinned sources.
"""

import json

import pytest

from pipeline import dk_encoding as enc
from pipeline.tests.markers import needs_sources


def pair(wid: str, k: str, d: str) -> enc.WordPair:
    """Clusters separated by spaces, e.g. "ba c" = two clusters "ba" and "c"."""
    return enc.WordPair(wid, tuple(k.split()), tuple(d.split()))


def rows_of(table: enc.Table) -> dict[tuple[str, ...], str]:
    return {r.src: r.dst for r in table.rows}


def test_one_to_one_clusters_become_rows_with_counts() -> None:
    t = enc.derive([pair("1", "a b", "A b"), pair("2", "b a", "b A")])
    assert rows_of(t) == {("a",): "A", ("b",): "b"}
    assert {r.src: r.count for r in t.rows} == {("a",): 2, ("b",): 2}
    assert t.differences == []


def test_a_form_with_two_counterparts_keeps_identity_and_reports_the_other() -> None:
    t = enc.derive([pair("1", "a", "a"), pair("2", "a", "a"), pair("3", "a", "aX")])
    assert rows_of(t) == {("a",): "a"}
    assert [d.word for d in t.differences] == ["3"]


def test_a_form_with_two_counterparts_and_no_identity_gets_no_row() -> None:
    t = enc.derive([pair("1", "a", "A"), pair("2", "a", "B")])
    assert rows_of(t) == {}
    assert t.ambiguous == {("a",): {"A": 1, "B": 1}}
    assert t.unmapped == {("a",): 2}


def test_a_cluster_that_becomes_two_is_a_sequence_row() -> None:
    t = enc.derive([pair("1", "a b", "a b"), pair("2", "a q b", "a Q1 Q2 b")])
    assert rows_of(t) == {("a",): "a", ("b",): "b", ("q",): "Q1Q2"}
    assert t.differences == []


def test_two_clusters_that_become_one_is_a_sequence_row() -> None:
    t = enc.derive([pair("1", "a b c", "a b c"), pair("2", "a b c x", "a Bx c")])
    assert rows_of(t)[("b", "c", "x")] == "Bxc"
    assert enc.convert(("a", "b", "c", "x"), t).text == "aBxc"


def test_sequence_row_that_breaks_another_word_is_rejected() -> None:
    pairs = [
        pair("1", "a b", "a b"),
        pair("2", "a q b", "a Q1 Q2 b"),
        pair("3", "c", "c"),
        pair("4", "q c", "q c"),  # here q is unchanged: the row would be wrong
    ]
    t = enc.derive(pairs)
    assert rows_of(t)[("q",)] == "q"
    assert t.rejected == {("q",): "Q1Q2"}
    assert [d.word for d in t.differences] == ["2"]


def test_word_level_insertions_are_differences_not_rows() -> None:
    t = enc.derive([pair("1", "a b", "a b"), pair("2", "a b", "a Z b")])
    assert rows_of(t) == {("a",): "a", ("b",): "b"}
    assert [d.word for d in t.differences] == ["2"]


def test_convert_maps_clusters_to_dk_char_ranges() -> None:
    t = enc.derive([pair("1", "a q b", "a Q1 Q2 b"), pair("2", "a b", "a b")])
    out = enc.convert(("a", "q", "b"), t)
    assert out.text == "aQ1Q2b"
    assert out.spans == [(0, 1, 0, 1), (1, 2, 1, 5), (2, 3, 5, 6)]


def test_convert_reports_unmapped_clusters() -> None:
    t = enc.derive([pair("1", "a", "a")])
    with pytest.raises(enc.EncodingError, match="no row"):
        enc.convert(("a", "z"), t)


def test_rows_sharing_one_output_lose_a_distinction_except_the_reference() -> None:
    # reference = the most used row of the group (no identity row here)
    t = enc.derive([pair("1", "a", "A"), pair("2", "a", "A"), pair("3", "aM", "A")])
    assert {r.src: r.lossy for r in t.rows} == {("a",): False, ("aM",): True}


def test_the_identity_row_is_the_reference_of_its_group() -> None:
    t = enc.derive([pair("1", "bM", "b"), pair("2", "bM", "b"), pair("3", "b", "b")])
    assert {r.src: r.lossy for r in t.rows} == {("b",): False, ("bM",): True}


def test_owner_rows_fill_forms_the_data_cannot_give() -> None:
    owner = (enc.Row(("t",), "t", 0, "owner"),)
    t = enc.derive([pair("1", "a", "a"), pair("2", "a t", "a")], owner)
    assert enc.convert(("a", "t"), t).text == "at"
    assert [d.word for d in t.differences] == ["2"]
    with pytest.raises(enc.EncodingError, match="owner row"):
        enc.derive([pair("1", "a", "a")], (enc.Row(("a",), "a", 0, "owner"),))


def test_owner_word_rows_take_the_aligned_word_for_that_item_only(tmp_path) -> None:
    pairs = [pair("1:1:1", "a t", "a t"), pair("1:1:2", "a t", "a T")]
    path = tmp_path / "words.json"
    path.write_text(json.dumps({"words": [{"item": "1:1:2", "decision": "test"}]}))
    t = enc.derive(pairs)
    t.word_rows = enc.owner_word_rows(pairs, path)
    assert t.word_rows == {"1:1:2": (("a", "t"), "aT", "test")}
    assert enc.convert_item("1:1:2", ("a", "t"), t).text == "aT"
    assert enc.convert_item("1:1:3", ("a", "t"), t).text == "at", "other items: cluster rows"
    again = enc.Table.from_json(json.loads(json.dumps(t.to_json(provenance={}))))
    assert again.word_rows == t.word_rows
    path.write_text(json.dumps({"words": [{"item": "9:9:9", "decision": "test"}]}))
    with pytest.raises(enc.EncodingError, match="not an aligned"):
        enc.owner_word_rows(pairs, path)


def test_table_round_trips_through_json() -> None:
    t = enc.derive([pair("1", "a q b", "a Q1 Q2 b"), pair("2", "a b", "a b")])
    again = enc.Table.from_json(json.loads(json.dumps(t.to_json(provenance={}))))
    assert again.rows == t.rows


def test_dk_clusterize_keeps_ayah_tokens_whole() -> None:
    table = {
        ord("a"): enc.DkRow(ord("a"), "base", "Ll", "A", "base", 1, ""),
        ord("m"): enc.DkRow(ord("m"), "mark", "Mn", "M", "mark", 1, ""),
        ord("#"): enc.DkRow(ord("#"), "ayah_number", "Cf", "H", "base", 1, ""),
        ord("1"): enc.DkRow(ord("1"), "digit", "Nd", "1", "base", 1, ""),
    }
    assert enc.dk_clusterize("ama", table) == ("am", "a")
    assert enc.dk_clusterize("#11", table) == ("#11",)
    with pytest.raises(enc.EncodingError):
        enc.dk_clusterize("ma", table)


# --- real data ----------------------------------------------------------------------------


@needs_sources
def test_dk_codepoint_table_is_current() -> None:
    assert enc.generate_dk_classes() == enc.load_dk_classes()


@needs_sources
def test_tracked_table_is_current_and_every_spine_cluster_converts() -> None:
    table, report = enc.build()
    assert enc.table_text(table) == enc.TABLE_PATH.read_text(encoding="ascii")
    assert report["unmapped"] == {}, "spine clusters with no conversion row"
    assert report["converted_words"] == report["spine_words"]


@needs_sources
def test_conversion_reproduces_digitalkhatt_text_except_listed_differences() -> None:
    _, report = enc.build()
    same = report["aligned_words"] - len(report["differences"])
    assert same == report["identical_after_conversion"]
    # every difference is listed in the review sheet
    sheet = enc.REVIEW_PATH.read_text(encoding="utf-8")
    for d in report["differences"]:
        assert d["word"] in sheet


@needs_sources
def test_every_digitalkhatt_basmalah_line_equals_the_converted_spine_basmalah() -> None:
    _, report = enc.build()
    assert report["basmalah_lines"] == 112  # QUL 1421H basmallah lines
    assert report["basmalah_lines_equal"] == report["basmalah_lines"]
