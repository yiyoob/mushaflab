"""D39: encoding rows for owner corrections to the Hafs spine (synthetic data, Latin letters)."""

import pytest

from pipeline import corrections
from pipeline.dk_encoding import Row, Table


def _table() -> Table:
    return Table([Row(("a\u0301",), "a\u0300", 1, "cluster", False)])


def _patch(cps: str) -> dict:
    return {
        "id": "p",
        "op": "replace",
        "target": "1:1:1:1",
        "payload": {"kind": "letter", "cps": cps},
    }


SPINE = {"1:1:1:1": "a\u0301"}


def test_row_takes_the_image_of_the_corrected_cluster() -> None:
    rows = corrections.rows([_patch("U+0061 U+0300")], SPINE, _table())
    assert rows == [
        {
            "src": ["U+0061 U+0300"],
            "dst": "U+0061 U+0300",
            "kind": "cluster",
            "groups": ["correction p"],
        }
    ]


def test_correction_must_equal_the_image_it_relies_on() -> None:
    # The evidence is that DigitalKhatt draws the same cluster as the corrected form.
    with pytest.raises(corrections.CorrectionError, match="image"):
        corrections.rows([_patch("U+0061 U+0302")], SPINE, _table())


def test_only_cluster_replacements_are_supported() -> None:
    bad = _patch("U+0061 U+0300") | {"op": "set_marks"}
    with pytest.raises(corrections.CorrectionError, match="replace"):
        corrections.rows([bad], SPINE, _table())


def test_a_form_the_table_already_has_needs_no_row() -> None:
    assert corrections.rows([_patch("U+0061 U+0301")], SPINE, _table()) == []
