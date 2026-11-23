//! `mushaf`: check and build the corpus (D69).
//!
//! ```text
//! mushaf check [--root <repo>]
//! mushaf build [--root <repo>] [--out <path>]   (default <repo>/build/corpus.db)
//! ```
//!
//! `check` loads `corpus/` and `data/approvals/corpus.yaml`, prints every error and the approval
//! state of each approved table, and exits 1 on any error. `build` runs the same check, then
//! writes a fresh SQLite file: same inputs, same rows, in the same order (rule 8; no timestamps).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mushaf_corpus::{Corpus, Files};
use rusqlite::{Connection, params};

fn usage() -> ExitCode {
    eprintln!("usage: mushaf check [--root <repo>] | mushaf build [--root <repo>] [--out <path>]");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else {
        return usage();
    };
    let mut root = PathBuf::from(".");
    let mut out: Option<PathBuf> = None;
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match (a.as_str(), it.next()) {
            ("--root", Some(v)) => root = PathBuf::from(v),
            ("--out", Some(v)) => out = Some(PathBuf::from(v)),
            _ => return usage(),
        }
    }
    let files = match read_files(&root) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("mushaf: {e}");
            return ExitCode::FAILURE;
        }
    };
    let corpus = match Corpus::load(&files) {
        Ok(c) => c,
        Err(errs) => {
            for e in &errs {
                eprintln!("error: {e}");
            }
            eprintln!("mushaf: {} error(s); nothing loaded", errs.len());
            return ExitCode::FAILURE;
        }
    };
    let report = corpus.check();
    for e in &report.errors {
        eprintln!("error: {e}");
    }
    println!(
        "corpus: {} sources, {} words, {} usul rows, {} adad rows, {} layout lines",
        corpus.sources.len(),
        corpus.words.len(),
        corpus.usul.len(),
        corpus.adad.len(),
        corpus.layout_lines.len()
    );
    for (table, state) in &report.approvals {
        println!("approval: {table}: {state}");
    }
    if !report.errors.is_empty() {
        eprintln!("mushaf: {} error(s)", report.errors.len());
        return ExitCode::FAILURE;
    }
    match cmd.as_str() {
        "check" => ExitCode::SUCCESS,
        "build" => {
            let out = out.unwrap_or_else(|| root.join("build/corpus.db"));
            match build(&corpus, &out) {
                Ok(()) => {
                    println!("built {}", out.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("mushaf: build: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => usage(),
    }
}

fn read_files(root: &Path) -> Result<Files, String> {
    mushaf_corpus::read_tree(root)
}

const SCHEMA: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE sources (id TEXT PRIMARY KEY, title TEXT NOT NULL, role TEXT NOT NULL, locator TEXT NOT NULL, pins TEXT NOT NULL);
CREATE TABLE words (ord INTEGER PRIMARY KEY, word_id TEXT NOT NULL UNIQUE, surah INTEGER NOT NULL, ayah INTEGER NOT NULL, word INTEGER NOT NULL,
  kind TEXT NOT NULL, text TEXT NOT NULL, sep_before TEXT NOT NULL, sep_after TEXT NOT NULL, source TEXT NOT NULL);
CREATE TABLE usul (file TEXT NOT NULL, word_id TEXT NOT NULL, letter INTEGER NOT NULL, category TEXT NOT NULL, context TEXT NOT NULL,
  exception TEXT NOT NULL, source TEXT NOT NULL, status TEXT NOT NULL, support TEXT NOT NULL, note TEXT NOT NULL, PRIMARY KEY (file, word_id, letter));
CREATE TABLE adad (file TEXT NOT NULL, surah INTEGER NOT NULL, ayah INTEGER NOT NULL, first_word TEXT NOT NULL, last_word TEXT NOT NULL,
  source TEXT NOT NULL, status TEXT NOT NULL, PRIMARY KEY (file, surah, ayah));
CREATE TABLE layout_lines (file TEXT NOT NULL, page INTEGER NOT NULL, line INTEGER NOT NULL, type TEXT NOT NULL, first_word TEXT, last_word TEXT,
  centered INTEGER NOT NULL, PRIMARY KEY (file, page, line));
CREATE TABLE layouts (layout_id TEXT PRIMARY KEY, file TEXT NOT NULL, script_id TEXT NOT NULL, edition TEXT NOT NULL, pages INTEGER NOT NULL,
  lines_per_page INTEGER NOT NULL, writing_style TEXT NOT NULL, font TEXT NOT NULL, sources TEXT NOT NULL);
CREATE TABLE approvals (table_path TEXT PRIMARY KEY, sha256 TEXT NOT NULL, status TEXT NOT NULL, by_whom TEXT NOT NULL, date TEXT NOT NULL,
  except_rows TEXT NOT NULL, state TEXT NOT NULL);
";

fn build(c: &Corpus, out: &Path) -> Result<(), String> {
    let e = |e: rusqlite::Error| e.to_string();
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = out.with_extension("db.tmp");
    let _ = std::fs::remove_file(&tmp);
    let mut db = Connection::open(&tmp).map_err(e)?;
    db.execute_batch(SCHEMA).map_err(e)?;
    let tx = db.transaction().map_err(e)?;
    tx.execute("INSERT INTO meta VALUES ('schema_version', '1')", [])
        .map_err(e)?;
    for (path, sha) in &c.hashes {
        tx.execute(
            "INSERT INTO meta VALUES (?1, ?2)",
            params![format!("sha256:{path}"), sha],
        )
        .map_err(e)?;
    }
    for s in &c.sources {
        tx.execute(
            "INSERT INTO sources VALUES (?1, ?2, ?3, ?4, ?5)",
            params![s.id, s.title, s.role, s.locator, s.pins.join(" ")],
        )
        .map_err(e)?;
    }
    for (n, w) in c.words.iter().enumerate() {
        tx.execute(
            "INSERT INTO words VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                n as i64,
                w.id.to_string(),
                w.id.surah,
                w.id.ayah,
                w.id.word,
                w.kind,
                w.text,
                w.sep_before,
                w.sep_after,
                w.source
            ],
        )
        .map_err(e)?;
    }
    for u in &c.usul {
        tx.execute(
            "INSERT INTO usul VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                u.file,
                u.word.to_string(),
                u.letter,
                u.category,
                u.context,
                u.exception,
                u.prov.source,
                u.prov.status,
                u.prov.support,
                u.prov.note
            ],
        )
        .map_err(e)?;
    }
    for a in &c.adad {
        tx.execute(
            "INSERT INTO adad VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                a.file,
                a.surah,
                a.ayah,
                a.first.to_string(),
                a.last.to_string(),
                a.source,
                a.status
            ],
        )
        .map_err(e)?;
    }
    for l in &c.layout_lines {
        tx.execute(
            "INSERT INTO layout_lines VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                l.file,
                l.page,
                l.line_no,
                l.kind,
                l.first.map(|w| w.to_string()),
                l.last.map(|w| w.to_string()),
                l.centered
            ],
        )
        .map_err(e)?;
    }
    for (path, l) in &c.layouts {
        let sources: Vec<&str> = l.sources.iter().map(|s| s.source.as_str()).collect();
        tx.execute(
            "INSERT INTO layouts VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                l.layout_id,
                path,
                l.script_id,
                l.edition,
                l.pages,
                l.lines_per_page,
                l.writing_style,
                l.font,
                sources.join(" ")
            ],
        )
        .map_err(e)?;
    }
    let report = c.check();
    for a in &c.approvals {
        let state = report.approvals.get(&a.table).cloned().unwrap_or_default();
        tx.execute(
            "INSERT INTO approvals VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                a.table,
                a.sha256,
                a.status,
                a.by,
                a.date,
                a.except.join(" "),
                state
            ],
        )
        .map_err(e)?;
    }
    tx.commit().map_err(e)?;
    drop(db);
    std::fs::rename(&tmp, out).map_err(|e| e.to_string())
}
