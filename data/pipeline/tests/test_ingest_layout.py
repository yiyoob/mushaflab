"""T1.3: the 1441H layout mapped onto the spine. Synthetic words use arbitrary letters."""

import pytest

from pipeline import ingest_layout as il
from pipeline.tests.markers import needs_sources

B, T, TH, J = "\u0628", "\u062a", "\u062b", "\u062c"


def test_align_one_to_one() -> None:
    m = il.align_ayah([("s1", B + T), ("s2", TH)], [(1, B + T), (2, TH)])
    assert m == {1: ["s1"], 2: ["s2"]}


def test_align_word_split_in_qul() -> None:
    m = il.align_ayah([("s1", B + T), ("s2", TH)], [(1, B), (2, T), (3, TH)])
    assert m == {1: ["s1"], 2: ["s1"], 3: ["s2"]}


def test_align_words_merged_in_qul() -> None:
    m = il.align_ayah([("s1", B), ("s2", T), ("s3", TH)], [(1, B + " " + T), (2, TH)])
    assert m == {1: ["s1", "s2"], 2: ["s3"]}


def test_align_rejects_different_letters() -> None:
    with pytest.raises(il.LayoutError, match="skeleton"):
        il.align_ayah([("s1", B + T)], [(1, B + J)])


def test_boundary_inside_a_spine_word_is_rejected() -> None:
    mapping = {1: ["s1"], 2: ["s1"], 3: ["s2"]}
    il.check_boundary(mapping, last_qid=2, next_qid=3)
    with pytest.raises(il.LayoutError, match="inside"):
        il.check_boundary(mapping, last_qid=1, next_qid=2)


@needs_sources
def test_committed_layout_is_current() -> None:
    assert il.load_layout() == il.build_layout()


@needs_sources
def test_layout_shape_and_reading_order() -> None:
    layout = il.load_layout()
    lines = layout["lines"]
    assert len({line["page"] for line in lines}) == layout["pages"]
    order = {r["id"]: n for n, r in enumerate(il.spine_records())}
    covered = []
    for line in lines:
        if line["type"] == "text":
            a, b = order[line["first"]], order[line["last"]]
            assert a <= b
            covered.extend(range(a, b + 1))
    assert covered == list(range(len(order)))  # every item exactly once, in order


@needs_sources
def test_every_text_difference_is_in_a_known_category() -> None:
    report = il.text_report()
    assert report["unexplained"] == []
    assert set(report["categories"]) <= set(il.KNOWN_DIFFERENCES) | {"exact"}


@needs_sources
def test_corrections_are_applied_with_provenance() -> None:
    corrected = [line for line in il.load_layout()["lines"] if "correction" in line]
    assert {(line["page"], line["line"]) for line in corrected} == {(254, 6), (254, 7)}
    assert all("owner print check" in line["correction"] for line in corrected)


@needs_sources
def test_qcf4_agrees_except_its_documented_page_92_error() -> None:
    assert il.qcf4_disagreements() == [(92, 12), (92, 13)]


@needs_sources
def test_kfgqpc_page_data_agrees_for_every_ayah() -> None:
    assert il.kfgqpc_page_disagreements() == []
