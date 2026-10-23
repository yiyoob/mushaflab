"""T5.3: candidate rows for the other Mashriqi riwayat (D28).

Synthetic tables use Latin letters (not Quranic text). Integration tests check the tracked
files against a rebuild, never against expected values about the Quran (rule 1).
"""

from collections import Counter

from pipeline import dk_riwayat
from pipeline.dk_encoding import Row, Table
from pipeline.tests.markers import needs_sources


def table() -> Table:
    rows = [
        Row(("ab",), "AB", 1, "cluster"),  # a -> A, b -> B
        Row(("cb",), "CB", 1, "cluster"),
        Row(("x",), "X1", 1, "cluster"),  # unequal length: no per-codepoint evidence
        Row(("da",), "DA", 1, "cluster"),
        Row(("dy",), "DY", 1, "cluster"),
        Row(("ey",), "EZ", 1, "cluster"),  # y has two images: Y and Z
    ]
    return Table(rows)


def test_images_come_only_from_equal_length_rows() -> None:
    img, bases = dk_riwayat.images(table())
    assert img["a"] == {"A"}
    assert img["y"] == {"Y", "Z"}
    assert "x" not in img
    assert bases["b"] == {"a", "c"}


def test_forms_are_composed_open_or_new_pair() -> None:
    t = table()
    img, bases = dk_riwayat.images(t)
    known = dk_riwayat.classify(("cb",), "cluster", t, img, bases)  # in the table already
    assert known.kind == "compose" and known.groups == ("compose: known pairs",)
    new = dk_riwayat.classify(("db",), "cluster", t, img, bases)
    assert new.dst == "DB"
    assert new.groups == ("mark U+0062 on new bases",)
    opened = dk_riwayat.classify(("dy",), "cluster", t, img, bases)
    assert opened.kind == "open" and opened.groups == ("open U+0079",)
    assert opened.evidence.startswith("NO EVIDENCE")


def test_rows_only_for_forms_whose_groups_are_all_approved() -> None:
    f1 = dk_riwayat.Form(("db",), "compose", "DB", ("g1",), "", Counter({"r": 1}))
    f2 = dk_riwayat.Form(("dbb",), "compose", "DBB", ("g1", "g2"), "", Counter({"r": 1}))
    forms = {f1.src: f1, f2.src: f2}
    owner = {"groups": [{"group": "g1"}], "rows": []}
    rows = dk_riwayat.rows_json(forms, owner)["rows"]
    assert [r["src"] for r in rows] == [["U+0064 U+0062"]]


def test_uncovered_respects_sequence_rows() -> None:
    t = Table([Row(("a",), "A", 1, "cluster"), Row(("q", "r"), "QR", 1, "sequence")])
    assert dk_riwayat.uncovered(("a", "q", "r", "z", "q"), t) == [3, 4]


@needs_sources
def test_tracked_files_are_current() -> None:
    assert dk_riwayat.main(["", "check"]) == 0
