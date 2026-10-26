"""T1.4/T2.2: build/mushaf.db is a deterministic, lossless compilation of the tracked files."""

import hashlib
import json
import sqlite3
from pathlib import Path

import pytest

from pipeline import compile_db, ingest_layout, spine
from pipeline.tests.markers import needs_sources


@needs_sources
def test_build_is_byte_identical(tmp_path: Path) -> None:
    a, b = tmp_path / "a.db", tmp_path / "b.db"
    compile_db.build(a)
    compile_db.build(b)
    assert hashlib.sha256(a.read_bytes()).digest() == hashlib.sha256(b.read_bytes()).digest()


@needs_sources
def test_db_round_trips_to_the_tracked_files(tmp_path: Path) -> None:
    db = tmp_path / "m.db"
    compile_db.build(db)
    assert compile_db.read_spine(db) == spine.load_records()
    assert compile_db.read_layout(db) == ingest_layout.load_layout()


@needs_sources
def test_schema_version_and_keys(tmp_path: Path) -> None:
    db = tmp_path / "m.db"
    compile_db.build(db)
    con = sqlite3.connect(db)
    meta = dict(con.execute("select key, value from meta"))
    assert meta["schema_version"] == "4"
    assert (
        meta["sha256:spine/hafs/v1/items.jsonl"]
        == hashlib.sha256(spine.ITEMS.read_bytes()).hexdigest()
    )
    assert "sha256:layouts/uthmani-1441h.json" in meta
    assert not any(k.endswith(".corrections.json") for k in meta)
    assert con.execute("select script_id from scripts").fetchall() == [("uthmani",)]
    native = ["bazzi", "duri_abuamr", "qalun", "qunbul", "shuba", "susi", "warsh"]
    assert con.execute(
        "select layout_id, script_id, riwayah from layouts order by layout_id"
    ).fetchall() == [("uthmani-1441h", "uthmani", None)] + [
        (f"uthmani-{r}-kfgqpc", "uthmani", r) for r in native
    ]
    con.close()


FIXTURE = Path(__file__).resolve().parents[3] / "tests" / "fixtures" / "synthetic"


def _rows(db: Path, sql: str) -> list[tuple]:
    con = sqlite3.connect(db)
    rows = con.execute(sql).fetchall()
    con.close()
    return rows


def test_synthetic_root_compiles(tmp_path: Path) -> None:
    db = tmp_path / "s.db"
    compile_db.build(db, FIXTURE)
    assert _rows(db, "select spine_id, script_id, version from spines") == [
        ("test/v1", "synthetic", 1)
    ]
    assert _rows(db, "select path, spine_id from patch_files order by path") == [
        ("patches/curated/q1.yaml", "test/v1"),
        ("patches/raw/r1.yaml", "test/v1"),
    ]
    assert _rows(db, "select id from option_groups") == [("test.other",), ("test.toggle",)]
    assert _rows(db, "select riwayah_id, spine_id from ayah_maps") == [("r1", "test/v1")]
    assert _rows(db, "select riwayah_id, spine_id from profiles") == [
        ("r1", "test/v1"),
        ("r3", "test/v1"),
    ]
    assert _rows(db, "select path, spine_id from rule_files") == [("rules/r3.yaml", "test/v1")]
    register = json.loads(_rows(db, "select body from rule_register")[0][0])
    assert [r["id"] for r in register["rules"]] == ["r3.mark", "r3.cross", "r3.toggle"]
    body = json.loads(_rows(db, "select body from patch_files where path like '%r1%'")[0][0])
    assert body["patches"][0]["target"] == "1:1:2:1"
    meta = dict(_rows(db, "select key, value from meta"))
    inputs = sorted(k.removeprefix("sha256:") for k in meta if k.startswith("sha256:"))
    assert inputs == [
        "ayah_maps/r1.yaml",
        "layouts/test-layout.json",
        "options/test.yaml",
        "patches/curated/q1.yaml",
        "patches/raw/r1.yaml",
        "profiles/r1.yaml",
        "profiles/r3.yaml",
        "rules/r3.yaml",
        "rules/register.yaml",
        "spine/test/v1/items.jsonl",
        "spine/test/v1/manifest.json",
    ]


def test_synthetic_build_is_byte_identical(tmp_path: Path) -> None:
    a, b = tmp_path / "a.db", tmp_path / "b.db"
    compile_db.build(a, FIXTURE)
    compile_db.build(b, FIXTURE)
    assert a.read_bytes() == b.read_bytes()


PATCH = """spine: "t/v1"
patches:
  - id: "p"
    owner: {level: "riwayah", id: "r"}
    op: "delete"
    target: "1:1:1:1"
    source: {kind: "test", ref: "x"}
    status: "raw-derived"
"""


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ('"1:1:1:1"', "1:1:1:1"),  # YAML 1.1: base-60 integer
        ('id: "r"', "id: yes"),  # YAML 1.1: boolean
        ('id: "r"', "id: 12"),
        ('id: "r"', "id: 1.5"),
        ('op: "delete"', 'op: "delete"\n    op: "delete"'),  # duplicate key
        ('source: {kind: "test", ref: "x"}', "source: {<<: {kind: x}, ref: x}"),  # merge key
        ('status: "raw-derived"', 'status: "raw-derived"\n    rule_id: 3'),
        ('id: "p"', 'id: "p\u00e9"'),  # non-ASCII patch file
    ],
)
def test_yaml_guards(tmp_path: Path, old: str, new: str) -> None:
    p = tmp_path / "x.yaml"
    p.write_text(PATCH, encoding="ascii")
    assert compile_db.load_yaml(p)["patches"][0]["target"] == "1:1:1:1"
    p.write_text(PATCH.replace(old, new), encoding="utf-8")
    with pytest.raises(compile_db.DataError):
        compile_db.load_yaml(p)


def test_option_files_may_hold_arabic_titles(tmp_path: Path) -> None:
    p = tmp_path / "o.yaml"
    p.write_text('- id: "g"\n  title_ar: "\u0627"\n  choices: ["a"]\n', encoding="utf-8")
    assert compile_db.load_yaml(p, ascii_only=False)[0]["title_ar"] == "\u0627"
