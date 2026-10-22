//! Host-only I/O for the core (D12): read a data root's files, or `mushaf.db` (schema v2), into
//! a `mushaf_model::DataSet`. All conversion from files or rows to the model lives in
//! `mushaf-model`; this crate only lists files and runs SELECTs. It is excluded from
//! `just core-targets` because bundled SQLite needs a C toolchain per target (G-032).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use mushaf_model::{
    BodyRow, ClusterRow, DATA_DIRS, DataFiles, DataSet, DbRows, Error, ItemRow, LayoutRow, LineRow,
    SpineRow, classify,
};
use rusqlite::{Connection, OpenFlags, Row};
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: &str = "2";

fn io(path: &Path, e: impl std::fmt::Display) -> Error {
    Error::new(format!("{}: {e}", path.display()))
}

fn walk(root: &Path, dir: &Path, out: &mut DataFiles) -> Result<(), Error> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| io(dir, e))?
        .map(|e| e.map(|e| e.path()).map_err(|e| io(dir, e)))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(root, &p, out)?;
        } else {
            let rel = p.strip_prefix(root).map_err(|e| io(&p, e))?;
            let key = rel
                .iter()
                .map(|c| c.to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(key, fs::read_to_string(&p).map_err(|e| io(&p, e))?);
        }
    }
    Ok(())
}

/// Every file under the data root's data directories (`spine/`, `layouts/`, ...).
pub fn read_tree(root: &Path) -> Result<DataFiles, Error> {
    let mut files = DataFiles::new();
    for d in DATA_DIRS {
        let dir = root.join(d);
        if dir.is_dir() {
            walk(root, &dir, &mut files)?;
        }
    }
    Ok(files)
}

pub fn load_dir(root: &Path) -> Result<DataSet, Error> {
    DataSet::from_files(&read_tree(root)?)
}

fn open(db: &Path) -> Result<Connection, Error> {
    let con =
        Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| io(db, e))?;
    let version = meta(&con)?.remove("schema_version").unwrap_or_default();
    if version != SCHEMA_VERSION {
        return Err(io(
            db,
            format!("schema version {version:?}, expected {SCHEMA_VERSION}"),
        ));
    }
    Ok(con)
}

fn meta(con: &Connection) -> Result<BTreeMap<String, String>, Error> {
    query(con, "select key, value from meta", |r| {
        Ok((r.get(0)?, r.get(1)?))
    })
    .map(|v| v.into_iter().collect())
}

fn query<T>(
    con: &Connection,
    sql: &str,
    f: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, Error> {
    let err = |e: rusqlite::Error| Error::new(format!("{sql}: {e}"));
    let mut stmt = con.prepare(sql).map_err(err)?;
    stmt.query_map([], f)
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)
}

pub fn load_db(db: &Path) -> Result<DataSet, Error> {
    let con = open(db)?;
    let rows = DbRows {
        spines: query(
            &con,
            "select spine_id, script_id, version from spines order by spine_id",
            |r| {
                Ok(SpineRow {
                    spine_id: r.get(0)?,
                    script_id: r.get(1)?,
                    version: r.get(2)?,
                })
            },
        )?,
        items: query(
            &con,
            "select spine_id, ord, id, kind, sep_before, sep_after from items order by spine_id, ord",
            |r| {
                Ok(ItemRow {
                    spine_id: r.get(0)?,
                    ord: r.get(1)?,
                    id: r.get(2)?,
                    kind: r.get(3)?,
                    sep_before: r.get(4)?,
                    sep_after: r.get(5)?,
                })
            },
        )?,
        clusters: query(
            &con,
            "select spine_id, item_id, idx, id, kind, text from clusters order by spine_id, item_id, idx",
            |r| {
                Ok(ClusterRow {
                    spine_id: r.get(0)?,
                    item_id: r.get(1)?,
                    idx: r.get(2)?,
                    id: r.get(3)?,
                    kind: r.get(4)?,
                    text: r.get(5)?,
                })
            },
        )?,
        layouts: query(
            &con,
            "select layout_id, script_id, spine_id, pages, lines_per_page, sources from layouts \
             order by layout_id",
            |r| {
                Ok(LayoutRow {
                    layout_id: r.get(0)?,
                    script_id: r.get(1)?,
                    spine_id: r.get(2)?,
                    pages: r.get(3)?,
                    lines_per_page: r.get(4)?,
                    sources: r.get(5)?,
                })
            },
        )?,
        lines: query(
            &con,
            "select layout_id, page, line, type, surah, first_item, last_item, centered, correction \
             from lines order by layout_id, page, line",
            |r| {
                Ok(LineRow {
                    layout_id: r.get(0)?,
                    page: r.get(1)?,
                    line: r.get(2)?,
                    kind: r.get(3)?,
                    surah: r.get(4)?,
                    first: r.get(5)?,
                    last: r.get(6)?,
                    centered: r.get::<_, i64>(7)? != 0,
                    correction: r.get(8)?,
                })
            },
        )?,
        patch_files: query(
            &con,
            "select path, spine_id, body from patch_files order by path",
            |r| {
                Ok(BodyRow {
                    key: r.get(0)?,
                    spine_id: r.get(1)?,
                    body: r.get(2)?,
                })
            },
        )?,
        option_groups: query(
            &con,
            "select id, body from option_groups order by id",
            |r| {
                Ok(BodyRow {
                    key: r.get(0)?,
                    spine_id: String::new(),
                    body: r.get(1)?,
                })
            },
        )?,
        ayah_maps: query(
            &con,
            "select riwayah_id, spine_id, body from ayah_maps order by riwayah_id",
            |r| {
                Ok(BodyRow {
                    key: r.get(0)?,
                    spine_id: r.get(1)?,
                    body: r.get(2)?,
                })
            },
        )?,
    };
    DataSet::from_rows(rows)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Fails unless the DB was compiled from exactly the data root's current input files: the
/// same set of files (`meta` keys `sha256:<path>`) with the same sha256.
pub fn check_current(db: &Path, root: &Path) -> Result<(), Error> {
    let recorded: BTreeMap<String, String> = meta(&open(db)?)?
        .into_iter()
        .filter_map(|(k, v)| k.strip_prefix("sha256:").map(|p| (p.to_string(), v)))
        .collect();
    let mut current = BTreeMap::new();
    for path in read_tree(root)?.keys() {
        if classify(path)?.is_some() {
            let bytes = fs::read(root.join(path)).map_err(|e| io(&root.join(path), e))?;
            current.insert(path.clone(), sha256_hex(&bytes));
        }
    }
    if recorded != current {
        let changed: Vec<&String> = recorded
            .keys()
            .chain(current.keys())
            .filter(|k| recorded.get(*k) != current.get(*k))
            .collect();
        return Err(io(
            db,
            format!("stale: run `just data-build` (changed: {changed:?})"),
        ));
    }
    Ok(())
}
