"""T3.2/T3.3: tracked render data is current and follows its rules."""

import json

from pipeline import line_widths, surah_names
from pipeline.tests.markers import needs_sources


@needs_sources
def test_line_widths_are_current() -> None:
    assert line_widths.text(line_widths.build()) == line_widths.OUT.read_text(encoding="ascii")


@needs_sources
def test_every_centred_line_is_short_except_listed_disagreements() -> None:
    data = json.loads(line_widths.OUT.read_text(encoding="ascii"))
    short = {(r["page"], r["line"]) for r in data["short_lines"]}
    disagree = {(r["page"], r["line"]) for r in data["centered_disagreements"]}
    layout = {(r["page"], r["line"]): r for r in line_widths.load_layout()["lines"]}
    centred = {k for k, r in layout.items() if r["type"] == "text" and r["centered"]}
    assert short ^ centred == disagree
    assert all(r["width_ppm"] < line_widths.SHORT_PPM for r in data["short_lines"])


@needs_sources
def test_surah_names_are_current() -> None:
    assert surah_names.text(surah_names.build()) == surah_names.OUT.read_text(encoding="ascii")
