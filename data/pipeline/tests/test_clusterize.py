"""T1.2: clusterizer. Synthetic cases use arbitrary letters, not Quranic text."""

import pytest

from pipeline.clusterize import ClusterError, clusterize
from pipeline.codepoints import TABLE_PATH, load_table
from pipeline.tests.markers import needs_sources

BEH, TEH, FATHA, SHADDA = "\u0628", "\u062a", "\u064e", "\u0651"
TATWEEL, SMALL_WAW, SAJDAH, RLM = "\u0640", "\u06e5", "\u06e9", "\u200f"
ZWJ, HAMZA_ABOVE = "\u200d", "\u0654"


@pytest.fixture(scope="module")
def table() -> dict:
    return load_table(TABLE_PATH)


@needs_sources
def test_base_takes_following_marks(table: dict) -> None:
    clusters = clusterize(BEH + SHADDA + FATHA + TEH, table)
    assert [c.text for c in clusters] == [BEH + SHADDA + FATHA, TEH]
    assert [c.kind for c in clusters] == ["letter", "letter"]


@needs_sources
def test_tatweel_and_small_waw_are_their_own_clusters(table: dict) -> None:
    clusters = clusterize(BEH + TATWEEL + FATHA + SMALL_WAW, table)
    assert [c.text for c in clusters] == [BEH, TATWEEL + FATHA, SMALL_WAW]


@needs_sources
def test_signs_and_controls_stand_alone(table: dict) -> None:
    clusters = clusterize(BEH + SAJDAH + RLM, table)
    assert [(c.text, c.kind) for c in clusters] == [
        (BEH, "letter"),
        (SAJDAH, "sign"),
        (RLM, "control"),
    ]


@needs_sources
def test_mark_without_base_is_an_error(table: dict) -> None:
    with pytest.raises(ClusterError, match="no base"):
        clusterize(FATHA + BEH, table)
    with pytest.raises(ClusterError, match="no base"):
        clusterize(SAJDAH + FATHA, table)


@needs_sources
def test_unknown_codepoint_is_an_error(table: dict) -> None:
    with pytest.raises(ClusterError, match="U\\+0041"):
        clusterize("A", table)


@needs_sources
def test_spaces_never_appear_inside_items(table: dict) -> None:
    with pytest.raises(ClusterError, match="space"):
        clusterize(BEH + " " + TEH, table)


@needs_sources
def test_joiner_keeps_the_cluster_open(table: dict) -> None:
    clusters = clusterize(BEH + ZWJ + HAMZA_ABOVE + TEH, table)
    assert [c.text for c in clusters] == [BEH + ZWJ + HAMZA_ABOVE, TEH]


@needs_sources
def test_every_token_of_all_8_texts_clusterizes() -> None:
    import re  # noqa: PLC0415

    from pipeline import kfgqpc  # noqa: PLC0415

    table = load_table(TABLE_PATH)
    for riwayah in kfgqpc.RIWAYAT:
        for row in kfgqpc.rows(riwayah):
            for token in re.split("[ \u00a0]", row["aya_text"]):
                if token:
                    clusters = clusterize(token, table)
                    assert "".join(c.text for c in clusters) == token
