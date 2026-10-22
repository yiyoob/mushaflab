"""T1.6: independent cross-check of the Hafs spine against Tanzil. Synthetic letters only."""

from pipeline import crosscheck
from pipeline.tests.markers import needs_sources

B, T, TH = "\u0628", "\u062a", "\u062b"


def test_identical_ayah_has_no_difference() -> None:
    assert crosscheck.compare([B + T, TH], [B + T, TH]) is None


def test_spacing_only_difference() -> None:
    assert crosscheck.compare([B + T, TH], [B, T, TH]) == "spacing"


def test_letter_difference() -> None:
    assert crosscheck.compare([B + T, TH], [B + TH, TH]) == "letters"


def test_marks_alone_do_not_count() -> None:
    assert crosscheck.compare([B + "\u064e" + T], [B + T + "\u0652"]) is None


@needs_sources
def test_every_ayah_is_compared() -> None:
    report = crosscheck.build_report()
    assert report["ayahs_compared"] == report["ayahs_in_spine"]


@needs_sources
def test_tanzil_differs_only_in_word_spacing() -> None:
    # Expected from spike S1 (docs/spikes/s1-editions.md, section 7): same letters in all
    # ayahs; word spacing differs in 15:7, 27:20 and 36:22. Owner adjudicates (G-030).
    report = crosscheck.build_report()
    assert {d["category"] for d in report["differences"]} <= {"spacing"}
    assert [d["ayah"] for d in report["differences"]] == ["15:7", "27:20", "36:22"]
