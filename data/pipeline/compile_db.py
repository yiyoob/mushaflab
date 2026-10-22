"""T1.4: compile the tracked spine and layout into build/mushaf.db (schema v1).

    python -m pipeline.compile_db [PATH]   # default build/mushaf.db

The DB is a build artifact (rule 3) holding exactly the tracked data, keyed by script_id
and layout_id. Rows are inserted in a fixed order into a fresh file, so the same inputs
give a byte-identical DB. Text is stored as real codepoints (the app's form); the tracked
files keep the ASCII "U+XXXX" form.
"""

import hashlib
import json
import sqlite3
import sys
from pathlib import Path

from pipeline import ingest_layout, spine
from pipeline.codepoints import TABLE_PATH

SCHEMA_VERSION = 1
DEFAULT_DB = Path(__file__).resolve().parents[2] / "build" / "mushaf.db"
SPINE_ID = f"hafs/v{spine.SPINE_VERSION}"

SCHEMA = """
create table meta (key text primary key, value text not null);
create table scripts (script_id text primary key);
create table spines (spine_id text primary key, script_id text not null, version integer not null);
create table items (
    spine_id text not null, ord integer not null, id text not null, kind text not null,
    sep_before text not null, sep_after text not null, primary key (spine_id, id));
create table clusters (
    spine_id text not null, item_id text not null, idx integer not null, id text not null,
    kind text not null, text text not null, primary key (spine_id, id));
create table layouts (
    layout_id text primary key, script_id text not null, spine_id text not null,
    pages integer not null, lines_per_page integer not null, sources text not null);
create table lines (
    layout_id text not null, page integer not null, line integer not null, type text not null,
    surah integer, first_item text, last_item text, centered integer not null,
    correction text, primary key (layout_id, page, line));
"""


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(db_path: Path = DEFAULT_DB) -> None:
    recs = spine.load_records()
    layout = ingest_layout.load_layout()
    db_path.parent.mkdir(parents=True, exist_ok=True)
    tmp = db_path.with_name(db_path.name + ".tmp")
    tmp.unlink(missing_ok=True)
    con = sqlite3.connect(tmp)
    con.executescript(SCHEMA)
    meta = {
        "schema_version": str(SCHEMA_VERSION),
        "spine_items_sha256": _sha256(spine.ITEMS),
        "spine_manifest_sha256": _sha256(spine.MANIFEST),
        "codepoint_table_sha256": _sha256(TABLE_PATH),
        "layout_sha256": _sha256(ingest_layout.LAYOUT_PATH),
    }
    con.executemany("insert into meta values (?, ?)", sorted(meta.items()))
    con.execute("insert into scripts values (?)", (spine.SCRIPT_ID,))
    con.execute(
        "insert into spines values (?, ?, ?)", (SPINE_ID, spine.SCRIPT_ID, spine.SPINE_VERSION)
    )
    for n, r in enumerate(recs):
        con.execute(
            "insert into items values (?, ?, ?, ?, ?, ?)",
            (
                SPINE_ID,
                n,
                r["id"],
                r["kind"],
                spine.from_cps(r.get("sep_before", "")),
                spine.from_cps(r["sep_after"]),
            ),
        )
        con.executemany(
            "insert into clusters values (?, ?, ?, ?, ?, ?)",
            [
                (SPINE_ID, r["id"], i, c["id"], c["kind"], spine.from_cps(c["cps"]))
                for i, c in enumerate(r["clusters"])
            ],
        )
    con.execute(
        "insert into layouts values (?, ?, ?, ?, ?, ?)",
        (
            layout["layout_id"],
            layout["script_id"],
            SPINE_ID,
            layout["pages"],
            layout["lines_per_page"],
            json.dumps(layout["sources"], sort_keys=True),
        ),
    )
    con.executemany(
        "insert into lines values (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        [
            (
                layout["layout_id"],
                ln["page"],
                ln["line"],
                ln["type"],
                ln.get("surah"),
                ln.get("first"),
                ln.get("last"),
                int(ln["centered"]),
                ln.get("correction"),
            )
            for ln in layout["lines"]
        ],
    )
    con.commit()
    con.execute("vacuum")
    con.close()
    tmp.replace(db_path)


def read_spine(db_path: Path) -> list[dict]:
    con = sqlite3.connect(db_path)
    clusters: dict[str, list[dict]] = {}
    for item_id, cid, kind, text in con.execute(
        "select item_id, id, kind, text from clusters where spine_id = ? order by item_id, idx",
        (SPINE_ID,),
    ):
        clusters.setdefault(item_id, []).append(
            {"id": cid, "kind": kind, "cps": spine.to_cps(text)}
        )
    recs = []
    for item_id, kind, before, after in con.execute(
        "select id, kind, sep_before, sep_after from items where spine_id = ? order by ord",
        (SPINE_ID,),
    ):
        rec: dict = {"id": item_id, "kind": kind}
        if before:
            rec["sep_before"] = spine.to_cps(before)
        rec["sep_after"] = spine.to_cps(after)
        rec["clusters"] = clusters.get(item_id, [])
        recs.append(rec)
    con.close()
    return recs


def read_layout(db_path: Path) -> dict:
    con = sqlite3.connect(db_path)
    layout_id, script_id, spine_id, pages, per_page, srcs = con.execute(
        "select layout_id, script_id, spine_id, pages, lines_per_page, sources from layouts"
    ).fetchone()
    lines = []
    for page, line, kind, surah, first, last, centered, correction in con.execute(
        "select page, line, type, surah, first_item, last_item, centered, correction"
        " from lines where layout_id = ? order by page, line",
        (layout_id,),
    ):
        rec: dict = {"page": page, "line": line, "type": kind}
        if surah is not None:
            rec["surah"] = surah
        if first is not None:
            rec["first"], rec["last"] = first, last
        rec["centered"] = bool(centered)
        if correction is not None:
            rec["correction"] = correction
        lines.append(rec)
    con.close()
    return {
        "layout_id": layout_id,
        "script_id": script_id,
        "spine": spine_id,
        "pages": pages,
        "lines_per_page": per_page,
        "sources": json.loads(srcs),
        "lines": lines,
    }


def main(argv: list[str]) -> int:
    db = Path(argv[1]) if len(argv) > 1 else DEFAULT_DB
    build(db)
    print(f"compile_db: {db} sha256 {_sha256(db)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
