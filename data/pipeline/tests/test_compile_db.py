"""T1.4: build/mushaf.db is a deterministic, lossless compilation of the tracked files."""

import hashlib
import sqlite3
from pathlib import Path

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
    assert meta["schema_version"] == "1"
    assert con.execute("select script_id from scripts").fetchall() == [("uthmani",)]
    assert con.execute("select layout_id, script_id from layouts").fetchall() == [
        ("uthmani-1441h", "uthmani")
    ]
    con.close()
