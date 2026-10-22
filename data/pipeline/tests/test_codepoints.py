"""T1.2: the codepoint class table (pipeline/codepoint_classes.tsv)."""

from pathlib import Path

import pytest

from pipeline.codepoints import (
    CLASSES,
    TABLE_PATH,
    CodepointError,
    Row,
    collect_evidence,
    expected_gdef,
    load_table,
    proposed_class,
    write_table,
)
from pipeline.tests.markers import needs_sources


def row(cp: int, cls: str, note: str = "") -> Row:
    return Row(cp, cls, "Lo", "TEST", "base:8", "hafs", note)


def test_table_round_trips(tmp_path: Path) -> None:
    rows = {0x0628: row(0x0628, "base"), 0x064E: row(0x064E, "mark")}
    p = tmp_path / "t.tsv"
    write_table(rows, p)
    assert load_table(p) == rows


def test_rejects_unknown_class(tmp_path: Path) -> None:
    p = tmp_path / "t.tsv"
    write_table({0x0628: row(0x0628, "letterish")}, p)
    with pytest.raises(CodepointError, match="class"):
        load_table(p)


def test_rejects_duplicate_rows(tmp_path: Path) -> None:
    p = tmp_path / "t.tsv"
    write_table({0x0628: row(0x0628, "base")}, p)
    lines = p.read_text(encoding="ascii").splitlines()
    p.write_text("\n".join([*lines, lines[-1]]) + "\n", encoding="ascii")
    with pytest.raises(CodepointError, match="duplicate"):
        load_table(p)


@pytest.mark.parametrize(
    ("cp", "category", "expected"),
    [
        (0x0628, "Lo", "base"),
        (0x064E, "Mn", "mark"),
        (0x06E5, "Lm", "base"),
        (0x06DE, "So", "sign"),
        (0x0661, "Nd", "digit"),
        (0x0020, "Zs", "space"),
        (0x200F, "Cf", "control"),
        (0xFC00, "Lo", "ayah_number"),
    ],
)
def test_proposed_class_follows_unicode_category(cp: int, category: str, expected: str) -> None:
    assert proposed_class(cp, category) == expected
    assert expected in CLASSES


def test_expected_gdef_for_each_class() -> None:
    assert expected_gdef("mark") == "mark"
    assert expected_gdef("base") == "base"
    assert expected_gdef("sign") == "base"


# --- the committed table against the pinned sources -----------------------------------


@needs_sources
def test_every_codepoint_in_all_8_texts_has_exactly_one_row() -> None:
    table = load_table(TABLE_PATH)
    assert set(collect_evidence()) == set(table)


@needs_sources
def test_table_evidence_is_current() -> None:
    table = load_table(TABLE_PATH)
    for cp, ev in collect_evidence().items():
        r = table[cp]
        assert (r.category, r.name, r.gdef, r.used_by) == ev, f"U+{cp:04X}"


@needs_sources
def test_rows_that_disagree_with_the_fonts_are_explained() -> None:
    for cp, r in load_table(TABLE_PATH).items():
        gdef_kinds = {part.split(":")[0] for part in r.gdef.split(",")}
        if gdef_kinds != {expected_gdef(r.cls)}:
            assert r.note, f"U+{cp:04X} {r.cls} vs GDEF {r.gdef} needs a note"
