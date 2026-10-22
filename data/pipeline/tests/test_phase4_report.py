"""T4.7: change buckets for the statistics (synthetic changes, Latin letters)."""

import pytest

from pipeline.differ import Change
from pipeline.phase4_report import bucket


def change(kind: str, item_kind: str = "word", level: str = "cluster") -> Change:
    return Change("1:1:1:1", "replace", {}, level, kind, item_kind, "a", "b", "1:1:1", 1)


@pytest.mark.parametrize(
    ("kind", "item_kind", "expected"),
    [
        ("marks", "word", "mark-only"),
        ("letter", "word", "letter"),
        ("cluster-insert", "word", "letter"),
        ("cluster-delete", "word", "letter"),
        ("item-insert", "word", "word"),
        ("item-delete", "word", "word"),
        ("separator", "word", "separator"),
        ("letter", "ayah_end", "ayah-number"),
        ("item-insert", "ayah_end", "ayah-number"),
        ("separator", "hizb", "hizb"),
    ],
)
def test_bucket(kind: str, item_kind: str, expected: str) -> None:
    assert bucket(change(kind, item_kind)) == expected
