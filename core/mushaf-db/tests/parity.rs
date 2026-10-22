//! T2.2 acceptance: loading from the tracked files and from `mushaf.db` gives identical
//! structures, for the synthetic fixture root (every op, an option group, an ayah map) and for
//! the real data root (Hafs spine and 1441H layout). The DBs are compiled by the Python
//! pipeline (`just core-fixtures`, `just data-build`), so this is a cross-implementation check.
//! Locally a missing DB skips with a message; CI sets MUSHAF_REQUIRE_SOURCES=1, which makes a
//! missing DB a failure.

use std::fs;
use std::path::{Path, PathBuf};

use mushaf_db::{check_current, load_db, load_dir};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The DB path, or None (skip) if it is absent and not required.
fn db(rel: &str, how: &str) -> Option<PathBuf> {
    let p = repo().join(rel);
    if p.exists() {
        return Some(p);
    }
    assert!(
        std::env::var_os("MUSHAF_REQUIRE_SOURCES").is_none(),
        "{rel} is missing: run `{how}`"
    );
    eprintln!("SKIPPED: {rel} is missing (run `{how}`)");
    None
}

#[test]
fn synthetic_files_and_db_are_identical() {
    let Some(db) = db("build/fixtures/synthetic.db", "just core-fixtures") else {
        return;
    };
    let root = repo().join("tests/fixtures/synthetic");
    check_current(&db, &root).unwrap();
    let (files, rows) = (load_dir(&root).unwrap(), load_db(&db).unwrap());
    assert_eq!(files.patch_files.len(), 2);
    assert_eq!(files, rows);
}

#[test]
fn real_files_and_db_are_identical() {
    let Some(db) = db("build/mushaf.db", "just data-build") else {
        return;
    };
    let root = repo().join("data");
    check_current(&db, &root).unwrap();
    let (files, rows) = (load_dir(&root).unwrap(), load_db(&db).unwrap());
    assert_eq!(files.spines["hafs/v1"].items.len(), 6236 + 199 + 77430); // spine manifest counts
    assert_eq!(files.layouts["uthmani-1441h"].lines.len(), 9046);
    assert_eq!(files, rows);
}

#[test]
fn spine_files_are_rewritten_byte_for_byte() {
    let root = repo().join("data");
    let text = fs::read_to_string(root.join("spine/hafs/v1/items.jsonl")).unwrap();
    let spine = &load_dir(&root).unwrap().spines["hafs/v1"];
    let rewritten: String = spine
        .items
        .iter()
        .map(|i| mushaf_model::to_json(i).unwrap() + "\n")
        .collect();
    assert!(
        rewritten == text,
        "Rust serialization of the spine differs from items.jsonl"
    );
}

#[test]
fn a_db_from_another_root_is_stale() {
    let Some(db) = db("build/fixtures/synthetic.db", "just core-fixtures") else {
        return;
    };
    let e = check_current(&db, &repo().join("data"))
        .unwrap_err()
        .to_string();
    assert!(e.contains("stale"), "{e}");
}
