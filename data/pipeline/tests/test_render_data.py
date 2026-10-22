"""T3.2/T3.3: tracked render data is current and follows its rules."""

import hashlib
import json

import pytest
from fontTools.ttLib import TTFont

from pipeline import derived_font, line_widths, surah_names
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


@needs_sources
def test_the_derived_render_font_is_current_and_patched_as_recorded() -> None:
    data, record = derived_font.build()
    assert derived_font.OUT.read_bytes() == data
    assert derived_font.record_text(record) == derived_font.RECORD.read_text(encoding="ascii")
    assert record["sha256"] == hashlib.sha256(data).hexdigest()
    patched = TTFont(derived_font.OUT)
    original = TTFont(derived_font.sources.path(derived_font.SOURCE))
    # only GSUB, name and head's whole-file checksum differ; outlines are untouched
    changed = {
        t
        for t in original.reader.tables
        if t != "GlyphOrder" and original.reader[t] != patched.reader[t]
    }
    assert changed == {"GSUB", "name", "head"}
    heads = original["head"].__dict__, patched["head"].__dict__
    assert {k for k in heads[0] if heads[0][k] != heads[1].get(k)} == {"checkSumAdjustment"}


def test_the_patch_is_refused_when_the_font_is_not_as_expected() -> None:
    font = TTFont(derived_font.OUT)  # already patched: no uni0670 -> glyph00103 left
    with pytest.raises(derived_font.PatchError, match="expected one"):
        derived_font.patch(font)
