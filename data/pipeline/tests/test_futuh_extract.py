"""al-Futuh glyph-level extraction on vol. 1 pdf p. 48. Needs the owner-supplied PDF and font
(MANIFEST.json, groups `books` and `owner-fonts`); skipped when they are not present."""

import pymupdf
import pytest

from pipeline.futuh.fontmap import FontMap
from pipeline.futuh.glyphs import InkBounds
from pipeline.futuh.page import PageModel, parse_page
from pipeline.futuh.paths import font_path
from pipeline.futuh.run import volume_pdf

try:
    PDF = volume_pdf(1)
except SystemExit:
    PDF = None

pytestmark = pytest.mark.skipif(
    PDF is None or not font_path().exists(), reason="owner-supplied PDF/font absent"
)


def page(n: int) -> PageModel:
    d = pymupdf.open(PDF)
    return parse_page(d, n - 1, InkBounds(d), {"SakkalMajalla": FontMap(font_path())})


def test_prose_roundtrip_has_no_failures() -> None:
    assert page(48).roundtrip["fail"] == 0


def test_rows_and_segments() -> None:
    rows = [r for r in page(48).rows if r.number.isdigit()]
    assert [r.number for r in rows] == ["1", "1", "2", "1", "1", "1", "2", "3"]  # printed numbers
    assert sum(r.group_start for r in rows) == 5


def test_no_orphan_marks() -> None:
    assert page(48).orphan_marks == 0
