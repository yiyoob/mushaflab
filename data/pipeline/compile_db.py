"""T1.4/T2.2: compile tracked data files into build/mushaf.db (schema v2).

    python -m pipeline.compile_db [--root DIR] [PATH]   # default: data/ -> build/mushaf.db

The DB is a build artifact (rule 3) holding exactly the tracked data under a data root:
spines (`spine/<name>/v<k>/`), layouts (`layouts/*.json`), patch files (`patches/**/*.yaml`),
option groups (`options/*.yaml`), ayah maps (`ayah_maps/*.yaml`) and riwayah profiles
(`profiles/*.yaml`, T5.1). Rows are inserted in a
fixed order into a fresh file, so the same inputs give a byte-identical DB. Spine text is
stored as real codepoints (the app's form); patch files, option groups and ayah maps as
canonical JSON, which the Rust core parses with the same types as the YAML files (D12).
`meta` records the sha256 of every input file as `sha256:<path relative to the root>`.

YAML is read strictly, because PyYAML implements YAML 1.1 and the core YAML 1.2 (D12):
duplicate and merge keys are errors, and every value must have the type the schema expects
(an unquoted `1:1:1` is a base-60 integer in YAML 1.1; `yes` is a boolean).
"""

import copy
import functools
import hashlib
import json
import sqlite3
import sys
from pathlib import Path

import yaml

from pipeline import spine
from pipeline.codepoints import TABLE_PATH

SCHEMA_VERSION = 3
DATA_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DB = DATA_ROOT.parent / "build" / "mushaf.db"
SPINE_ID = f"hafs/v{spine.SPINE_VERSION}"

# Keys whose values are not strings (every other scalar must be a string).
INT_KEYS = frozenset(
    {
        "seq",
        "at",
        "width_em_delta",
        "stretch",
        "surah",
        "ayah",
        "page",
        "name_line",
        "basmalah_line",
    }
)
BOOL_KEYS = frozenset({"single_source"})
NULL_KEYS = frozenset({"rule_id"})
MERGE_TAG = "tag:yaml.org,2002:merge"

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
create table patch_files (path text primary key, spine_id text not null, body text not null);
create table option_groups (id text primary key, body text not null);
create table ayah_maps (riwayah_id text primary key, spine_id text not null, body text not null);
create table profiles (riwayah_id text primary key, spine_id text not null, body text not null);
"""


class DataError(ValueError):
    pass


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


# libyaml's parser when available (the raw patch files are tens of MB, D20); the strict
# checks below are the same either way.
_BaseLoader = getattr(yaml, "CSafeLoader", yaml.SafeLoader)


class _StrictLoader(_BaseLoader):  # type: ignore[misc,valid-type]
    pass


def _construct_mapping(loader: _StrictLoader, node: yaml.MappingNode) -> dict:
    seen = set()
    for key_node, _ in node.value:
        if key_node.tag == MERGE_TAG:
            raise DataError(f"merge key at line {key_node.start_mark.line + 1}")
        key = loader.construct_object(key_node)
        if key in seen:
            raise DataError(f"duplicate key {key!r} at line {key_node.start_mark.line + 1}")
        seen.add(key)
    return loader.construct_mapping(node)


_StrictLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _construct_mapping)


def _check_types(value: object, key: str, where: str) -> None:
    if isinstance(value, dict):
        for k, v in value.items():
            if not isinstance(k, str):
                raise DataError(f"{where}: key {k!r} is not a string")
            _check_types(v, k, where)
    elif isinstance(value, list):
        for v in value:
            _check_types(v, key, where)
    elif isinstance(value, bool):
        if key not in BOOL_KEYS:
            raise DataError(f"{where}: {key} = {value!r} must be a quoted string")
    elif isinstance(value, int):
        if key not in INT_KEYS:
            raise DataError(f"{where}: {key} = {value!r} must be a quoted string")
    elif value is None:
        if key not in NULL_KEYS:
            raise DataError(f"{where}: {key} is null")
    elif not isinstance(value, str):
        raise DataError(f"{where}: {key} = {value!r} has unsupported type {type(value).__name__}")


def load_yaml(path: Path, ascii_only: bool = True) -> object:
    text = path.read_text(encoding="utf-8")
    if ascii_only and not text.isascii():
        raise DataError(f"{path}: must be ASCII (codepoints are written as U+XXXX)")
    return copy.deepcopy(_parse_yaml(text, str(path)))


@functools.lru_cache(maxsize=32)
def _parse_yaml(text: str, where: str) -> object:
    """Parsed once per content and process: the raw patch files are tens of MB (D20)."""
    data = yaml.load(text, Loader=_StrictLoader)  # noqa: S506 (a SafeLoader subclass)
    _check_types(data, "", where)
    return data


def canonical_json(data: object) -> str:
    return json.dumps(data, ensure_ascii=True, separators=(",", ":"))


def _rel(path: Path, root: Path) -> str:
    return path.relative_to(root).as_posix()


def spine_dirs(root: Path) -> list[Path]:
    return sorted(p.parent for p in root.glob("spine/*/v*/manifest.json"))


def layout_files(root: Path) -> list[Path]:
    paths = (root / "layouts").glob("*.json")
    return sorted(p for p in paths if not p.name.endswith(".corrections.json"))


def yaml_files(root: Path, sub: str) -> list[Path]:
    return sorted((root / sub).rglob("*.yaml"))


def _load_spine(d: Path) -> tuple[str, dict, list[dict]]:
    manifest = json.loads((d / "manifest.json").read_text(encoding="ascii"))
    if d.name != f"v{manifest['spine_version']}":
        raise DataError(f"{d}: directory does not match spine_version {manifest['spine_version']}")
    lines = (d / "items.jsonl").read_text(encoding="ascii").splitlines()
    return f"{d.parent.name}/{d.name}", manifest, [json.loads(line) for line in lines]


def build(db_path: Path = DEFAULT_DB, root: Path = DATA_ROOT) -> None:
    root = root.resolve()
    spines = [_load_spine(d) for d in spine_dirs(root)]
    layouts = [(p, json.loads(p.read_text(encoding="ascii"))) for p in layout_files(root)]
    patch_files = [(p, load_yaml(p)) for p in yaml_files(root, "patches")]
    option_files = [(p, load_yaml(p, ascii_only=False)) for p in yaml_files(root, "options")]
    ayah_maps = [(p, load_yaml(p)) for p in yaml_files(root, "ayah_maps")]
    profiles = [(p, load_yaml(p)) for p in yaml_files(root, "profiles")]

    inputs = [d / f for d in spine_dirs(root) for f in ("items.jsonl", "manifest.json")]
    inputs += [p for p, _ in layouts + patch_files + option_files + ayah_maps + profiles]

    db_path.parent.mkdir(parents=True, exist_ok=True)
    tmp = db_path.with_name(db_path.name + ".tmp")
    tmp.unlink(missing_ok=True)
    con = sqlite3.connect(tmp)
    con.executescript(SCHEMA)
    meta = {"schema_version": str(SCHEMA_VERSION), "codepoint_table_sha256": _sha256(TABLE_PATH)}
    meta |= {f"sha256:{_rel(p, root)}": _sha256(p) for p in inputs}
    con.executemany("insert into meta values (?, ?)", sorted(meta.items()))
    scripts = sorted({m["script_id"] for _, m, _ in spines})
    con.executemany("insert into scripts values (?)", [(s,) for s in scripts])
    for spine_id, manifest, recs in spines:
        _insert_spine(con, spine_id, manifest, recs)
    for _, layout in layouts:
        _insert_layout(con, layout)
    for p, data in patch_files:
        if not isinstance(data, dict) or not isinstance(data.get("spine"), str):
            raise DataError(f"{p}: a patch file is a mapping with a `spine` id")
        con.execute(
            "insert into patch_files values (?, ?, ?)",
            (_rel(p, root), data["spine"], canonical_json(data)),
        )
    groups: dict[str, str] = {}
    for p, data in option_files:
        if not isinstance(data, list):
            raise DataError(f"{p}: an option file is a list of option groups")
        for g in data:
            if not isinstance(g, dict) or not isinstance(g.get("id"), str) or g["id"] in groups:
                raise DataError(f"{p}: option group without an id, or a duplicate id")
            groups[g["id"]] = canonical_json(g)
    con.executemany("insert into option_groups values (?, ?)", sorted(groups.items()))
    seen: set[str] = set()
    for p, data in ayah_maps:
        if not isinstance(data, dict) or data.get("riwayah") in seen:
            raise DataError(f"{p}: ayah map without a riwayah, or a duplicate riwayah")
        seen.add(data["riwayah"])
        con.execute(
            "insert into ayah_maps values (?, ?, ?)",
            (data["riwayah"], data["spine"], canonical_json(data)),
        )
    seen = set()
    for p, data in profiles:
        if not isinstance(data, dict) or data.get("riwayah") in seen:
            raise DataError(f"{p}: profile without a riwayah, or a duplicate riwayah")
        seen.add(data["riwayah"])
        con.execute(
            "insert into profiles values (?, ?, ?)",
            (data["riwayah"], data["spine"], canonical_json(data)),
        )
    con.commit()
    con.execute("vacuum")
    con.close()
    tmp.replace(db_path)


def _insert_spine(con: sqlite3.Connection, spine_id: str, manifest: dict, recs: list) -> None:
    con.execute(
        "insert into spines values (?, ?, ?)",
        (spine_id, manifest["script_id"], manifest["spine_version"]),
    )
    for n, r in enumerate(recs):
        con.execute(
            "insert into items values (?, ?, ?, ?, ?, ?)",
            (
                spine_id,
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
                (spine_id, r["id"], i, c["id"], c["kind"], spine.from_cps(c["cps"]))
                for i, c in enumerate(r["clusters"])
            ],
        )


def _insert_layout(con: sqlite3.Connection, layout: dict) -> None:
    con.execute(
        "insert into layouts values (?, ?, ?, ?, ?, ?)",
        (
            layout["layout_id"],
            layout["script_id"],
            layout["spine"],
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
    args = argv[1:]
    root = DATA_ROOT
    if args[:1] == ["--root"]:
        root, args = Path(args[1]), args[2:]
    db = Path(args[0]) if args else DEFAULT_DB
    build(db, root)
    print(f"compile_db: {db} sha256 {_sha256(db)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
