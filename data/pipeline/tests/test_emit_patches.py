"""T4.5 / D21: raw patch files and difference-kind tags.

Synthetic changes use Latin letters (not Quranic text). The integration test compares the
tracked Shuba file with a rebuild from the pinned sources.
"""

from pathlib import Path

import pytest

from pipeline import classify, emit_patches
from pipeline.classify import RuleError, Tag, components, load_confirmed
from pipeline.compile_db import load_yaml
from pipeline.differ import Change
from pipeline.tests.markers import needs_sources


def change(kind: str = "marks", old: str = "a\u0301", new: str = "a\u0300", **kw: object) -> Change:
    fields = {
        "target": "1:1:1:1",
        "op": "set_marks",
        "payload": {"remove": ["U+0301"], "add": ["U+0300"]},
        "level": "cluster",
        "kind": kind,
        "item_kind": "word",
        "old": old,
        "new": new,
        "ref": "1:1:1",
        "surah": 1,
    }
    return Change(**(fields | kw))  # type: ignore[arg-type]


def test_components_split_substitutions_from_other_spans() -> None:
    assert components("a\u0301", "a\u0300") == [("sub", "\u0301", "\u0300")]
    assert components("ab", "abc") == [("insert", "", "c")]
    assert components("abc", "ac") == [("delete", "b", "")]


def test_default_is_unclassified() -> None:
    assert classify.classify(change(), {}) == Tag("unclassified", "default")


def test_ayah_end_changes_have_their_own_rule() -> None:
    c = change(item_kind="ayah_end")
    assert classify.classify(c, {("\u0301", "\u0300"): "encoding"}) == Tag(
        "unclassified", "ayah-number"
    )


def test_confirmed_rows_tag_only_fully_explained_changes() -> None:
    rows = {("\u0301", "\u0300"): "encoding"}
    assert classify.classify(change(), rows) == Tag("encoding", "confirmed")
    mixed = change(old="a\u0301b", new="a\u0300c")
    assert classify.classify(mixed, rows).kind == "unclassified"
    extra = change(old="a\u0301", new="a\u0300\u0302")
    assert classify.classify(extra, rows).kind == "unclassified"
    two = {**rows, ("b", "c"): "reading"}
    assert classify.classify(mixed, two).kind == "unclassified"  # two kinds: not tagged


def test_item_changes_are_never_confirmed() -> None:
    sep = change(level="item", kind="separator", old=" ", new="\u00a0")
    assert classify.classify(sep, {(" ", "\u00a0"): "encoding"}).kind == "unclassified"


def test_rows_come_from_the_owner_signed_approvals_dir(tmp_path: Path) -> None:
    assert classify.RULES_PATH.parent.name == "approvals"
    assert all(rows == {} for rows in load_confirmed(tmp_path / "missing.yaml").values())
    load_confirmed()  # the tracked file, if the owner has committed it, must be valid


def test_a_valid_row_is_read(tmp_path: Path) -> None:
    p = tmp_path / "kinds.yaml"
    p.write_text(
        'confirmed:\n  - {riwayah: "warsh", from: "U+06E1", to: "U+0652", kind: "encoding",'
        ' by: "o", date: "2026-10-22", evidence: "stats.md"}\n',
        encoding="ascii",
    )
    assert load_confirmed(p)["warsh"] == {("\u06e1", "\u0652"): "encoding"}
    assert load_confirmed(p)["qalun"] == {}


@pytest.mark.parametrize(
    "body",
    [
        "rules: []\n",
        'confirmed:\n  - {riwayah: "warsh", from: "U+06E1", to: "U+0652", kind: "encoding"}\n',
        'confirmed:\n  - {riwayah: "x", from: "U+06E1", to: "U+0652", kind: "encoding",'
        ' by: "o", date: "d", evidence: "e"}\n',
        'confirmed:\n  - {riwayah: "warsh", from: "U+06E1 U+0652", to: "U+0652",'
        ' kind: "encoding", by: "o", date: "d", evidence: "e"}\n',
        'confirmed:\n  - {riwayah: "warsh", from: "U+06E1", to: "U+0652",'
        ' kind: "unclassified", by: "o", date: "d", evidence: "e"}\n',
    ],
)
def test_bad_rule_tables_are_rejected(tmp_path: Path, body: str) -> None:
    p = tmp_path / "kinds.yaml"
    p.write_text(body, encoding="ascii")
    with pytest.raises(RuleError):
        load_confirmed(p)


def test_flow_lines_parse_back_with_the_strict_loader(tmp_path: Path) -> None:
    d = emit_patches.patch_dict("r", change(), "src@0123", Tag("unclassified", "default"))
    p = tmp_path / "p.yaml"
    p.write_text(f'spine: "hafs/v1"\npatches:\n  - {emit_patches.flow(d)}\n', encoding="ascii")
    assert load_yaml(p) == {"spine": "hafs/v1", "patches": [d]}
    assert d["id"] == "r.1.1.1.1.marks"
    assert (d["status"], d["single_source"], d["difference_kind"]) == (
        "raw-derived",
        True,
        "unclassified",
    )


def test_insert_payloads_keep_int_seq() -> None:
    c = change(
        op="insert_after",
        target="1:1:2",
        payload={"seq": 2, "item": {"kind": "word", "sep_after": "U+0020", "clusters": []}},
        level="item",
        kind="item-insert",
    )
    line = emit_patches.flow(emit_patches.patch_dict("r", c, "s", Tag("unclassified", "default")))
    assert '"seq": 2' not in line
    assert "seq: 2" in line
    assert '"id": ' not in line
    assert 'id: "r.1.1.2+ins.2.ins"' in line


@needs_sources
def test_tracked_shuba_file_is_current() -> None:
    assert emit_patches.path_for("shuba").read_text() == emit_patches.render("shuba")
