//! M0.3: `mushaf check` and `mushaf build` end to end, on the synthetic fixture
//! (`tests/fixtures/corpus/`) and on the repository's own `corpus/`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn mushaf(args: &[&str]) -> (bool, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_mushaf"))
        .args(args)
        .output()
        .unwrap();
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn the_fixture_checks_and_builds() {
    let root = repo().join("tests/fixtures/corpus");
    let (ok, out, err) = mushaf(&["check", "--root", root.to_str().unwrap()]);
    assert!(ok, "{out}{err}");
    assert!(out.contains("2 sources, 4 words, 2 usul rows"), "{out}");

    let db = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fixture-corpus.db");
    let (ok, out, err) = mushaf(&[
        "build",
        "--root",
        root.to_str().unwrap(),
        "--out",
        db.to_str().unwrap(),
    ]);
    assert!(ok, "{out}{err}");
    let con = rusqlite::Connection::open(&db).unwrap();
    let n: i64 = con
        .query_row("select count(*) from words", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 4);
    let n: i64 = con
        .query_row("select count(*) from usul", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);

    // Same inputs, same file (rule 8).
    let first = std::fs::read(&db).unwrap();
    let (ok, ..) = mushaf(&[
        "build",
        "--root",
        root.to_str().unwrap(),
        "--out",
        db.to_str().unwrap(),
    ]);
    assert!(ok);
    assert_eq!(first, std::fs::read(&db).unwrap());
}

#[test]
fn the_repository_corpus_checks() {
    let (ok, out, err) = mushaf(&["check", "--root", repo().to_str().unwrap()]);
    assert!(ok, "{out}{err}");
}

#[test]
fn errors_exit_non_zero() {
    let (ok, _, err) = mushaf(&["check", "--root", "/nonexistent-mushaf-root"]);
    assert!(!ok);
    assert!(err.contains("nonexistent"), "{err}");
}
