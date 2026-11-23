//! The corpus (D67, D69): load the text tables under `corpus/` and check them.
//!
//! Pure: callers pass file contents by repository-relative path (`Files`), so the same code
//! runs on the host (`mushaf check`, `mushaf build` in `mushaf-cli`) and, later, in WASM.
//! Formats: `docs/formats.md`. M0.3 loads the tables whose format is settled (sources, words,
//! uṣūl, ayah counting, layout lines) and the approvals file; farsh, riwāyah profiles and writing
//! styles arrive with the Warsh prototype (M0.6).

mod tsv;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;
use sha2::{Digest, Sha256};

pub use tsv::{Row, Table};

/// File contents by repository-relative path with `/` separators (`corpus/text/words.tsv`).
pub type Files = BTreeMap<String, String>;

pub const SOURCES: &str = "corpus/sources.yaml";
pub const WORDS: &str = "corpus/text/words.tsv";
pub const APPROVALS: &str = "data/approvals/corpus.yaml";

/// Statuses a data file may carry. `cross-checked` and `muqri-verified` come only from approvals
/// (CLAUDE.md rule 4).
pub const STATUSES: [&str; 3] = ["raw-derived", "rule-derived", "hypothesis"];
pub const OWNER_STATUSES: [&str; 2] = ["cross-checked", "muqri-verified"];
pub const SUPPORT: [&str; 2] = ["cross-source", "single-source"];

/// One problem, at a file and (when known) a 1-based line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diag {
    pub file: String,
    pub line: Option<usize>,
    pub msg: String,
}

impl Diag {
    fn new(file: &str, line: Option<usize>, msg: impl Into<String>) -> Self {
        Diag {
            file: file.to_string(),
            line,
            msg: msg.into(),
        }
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "{}:{}: {}", self.file, l, self.msg),
            None => write!(f, "{}: {}", self.file, self.msg),
        }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A word address `surah:ayah:word`, ordered numerically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WordId {
    pub surah: u16,
    pub ayah: u16,
    pub word: u16,
}

impl WordId {
    /// Parse `surah:ayah:word`. Surah and ayah are 1-based; the word position may be 0, which
    /// the word table uses only for a ḥizb sign before an ayah's first word (D11).
    pub fn parse(s: &str) -> Option<WordId> {
        let mut it = s.split(':').map(number);
        let w = WordId {
            surah: it.next()?.filter(|n| *n > 0)?,
            ayah: it.next()?.filter(|n| *n > 0)?,
            word: it.next()??,
        };
        it.next().is_none().then_some(w)
    }
}

impl fmt::Display for WordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.surah, self.ayah, self.word)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub edition: Option<String>,
    pub role: String,
    pub locator: String,
    pub pins: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcesFile {
    sources: Vec<Source>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub table: String,
    pub sha256: String,
    pub status: String,
    pub by: String,
    pub date: String,
    #[serde(default)]
    pub except: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub id: WordId,
    pub kind: String,
    pub text: String,
    pub sep_before: String,
    pub sep_after: String,
    pub source: String,
    pub line: usize,
}

/// Provenance cells shared by fact rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub source: String,
    pub status: String,
    pub support: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsulRow {
    pub file: String,
    pub line: usize,
    pub word: WordId,
    pub letter: u16,
    pub category: String,
    pub context: String,
    pub exception: String,
    pub prov: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdadRow {
    pub file: String,
    pub line: usize,
    pub surah: u16,
    pub ayah: u16,
    pub first: WordId,
    pub last: WordId,
    pub source: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutLine {
    pub file: String,
    pub line: usize,
    pub page: u16,
    pub line_no: u16,
    pub kind: String,
    pub first: Option<WordId>,
    pub last: Option<WordId>,
    pub centered: bool,
}

/// Everything loaded from `corpus/` (plus the approvals file).
#[derive(Debug, Default)]
pub struct Corpus {
    pub sources: Vec<Source>,
    pub words: Vec<Word>,
    pub usul: Vec<UsulRow>,
    pub adad: Vec<AdadRow>,
    pub layout_lines: Vec<LayoutLine>,
    pub approvals: Vec<Approval>,
    /// sha256 of every corpus file, by path (for approvals and the build's meta table).
    pub hashes: BTreeMap<String, String>,
    /// Row checks that only need one file, found while loading.
    load_errors: Vec<Diag>,
}

/// The result of `Corpus::check`.
#[derive(Debug, Default)]
pub struct Report {
    pub errors: Vec<Diag>,
    /// Approval state per approved table: `current` or `stale`.
    pub approvals: BTreeMap<String, String>,
}

const WORD_HEADER: [&str; 6] = [
    "word_id",
    "kind",
    "text",
    "sep_before",
    "sep_after",
    "source",
];
const USUL_HEADER: [&str; 9] = [
    "word_id",
    "letter",
    "category",
    "context",
    "exception",
    "source",
    "status",
    "support",
    "note",
];
const ADAD_HEADER: [&str; 6] = [
    "surah",
    "ayah",
    "first_word",
    "last_word",
    "source",
    "status",
];
const LINES_HEADER: [&str; 6] = [
    "page",
    "line",
    "type",
    "first_word",
    "last_word",
    "centered",
];
const WORD_KINDS: [&str; 3] = ["word", "ayah_end", "hizb"];
const CONTEXTS: [&str; 3] = ["wasl", "waqf", "both"];

/// A decimal number cell: digits only, no sign, no leading zero (except "0" itself).
fn number(cell: &str) -> Option<u16> {
    let digits = !cell.is_empty() && cell.bytes().all(|b| b.is_ascii_digit());
    (digits && (!cell.starts_with('0') || cell == "0"))
        .then(|| cell.parse::<u16>().ok())
        .flatten()
}

impl Corpus {
    /// Parse every corpus file. A file that cannot be read as its format (YAML syntax, a TSV
    /// header) fails the load; row-level problems are reported by `check`.
    pub fn load(files: &Files) -> Result<Corpus, Vec<Diag>> {
        let mut c = Corpus::default();
        let mut fatal = Vec::new();
        for (path, text) in files {
            if path.starts_with("corpus/") {
                c.hashes.insert(path.clone(), sha256_hex(text.as_bytes()));
            }
        }
        match files.get(SOURCES) {
            None => fatal.push(Diag::new(SOURCES, None, "missing")),
            Some(t) => match mushaf_model::from_yaml::<SourcesFile>(t) {
                Ok(s) => c.sources = s.sources,
                Err(e) => fatal.push(Diag::new(SOURCES, None, e.to_string())),
            },
        }
        if let Some(t) = files.get(APPROVALS) {
            match mushaf_model::from_yaml::<Vec<Approval>>(t) {
                Ok(a) => c.approvals = a,
                Err(e) => fatal.push(Diag::new(APPROVALS, None, e.to_string())),
            }
        }
        for (path, text) in files {
            let table = |header: &[&str]| tsv::parse(path, text, header);
            let res = if path == WORDS {
                table(&WORD_HEADER).map(|t| c.load_words(&t))
            } else if let Some(name) = tsv_under(path, "corpus/usul/") {
                table(&USUL_HEADER).map(|t| c.load_usul(&t, name))
            } else if tsv_under(path, "corpus/adad/").is_some() {
                table(&ADAD_HEADER).map(|t| c.load_adad(&t))
            } else if path.starts_with("corpus/layouts/") && path.ends_with("/lines.tsv") {
                table(&LINES_HEADER).map(|t| c.load_lines(&t))
            } else {
                Ok(())
            };
            if let Err(d) = res {
                fatal.extend(d);
            }
        }
        if fatal.is_empty() { Ok(c) } else { Err(fatal) }
    }

    fn err(&mut self, file: &str, line: usize, msg: impl Into<String>) {
        self.load_errors.push(Diag::new(file, Some(line), msg));
    }

    fn word_id(&mut self, file: &str, line: usize, cell: &str) -> Option<WordId> {
        let w = WordId::parse(cell);
        if w.is_none() {
            self.err(file, line, format!("bad word id {cell:?}"));
        }
        w
    }

    fn load_words(&mut self, t: &Table) {
        for r in &t.rows {
            let Some(id) = self.word_id(&t.path, r.line, &r.cells[0]) else {
                continue;
            };
            if !WORD_KINDS.contains(&r.cells[1].as_str()) {
                self.err(&t.path, r.line, format!("unknown kind {:?}", r.cells[1]));
            }
            if (id.word == 0) != (r.cells[1] == "hizb") {
                self.err(
                    &t.path,
                    r.line,
                    format!("{id}: position 0 is for hizb items only, and every hizb item"),
                );
            }
            self.words.push(Word {
                id,
                kind: r.cells[1].clone(),
                text: r.cells[2].clone(),
                sep_before: r.cells[3].clone(),
                sep_after: r.cells[4].clone(),
                source: r.cells[5].clone(),
                line: r.line,
            });
        }
    }

    fn load_usul(&mut self, t: &Table, name: &str) {
        for r in &t.rows {
            let Some(word) = self.word_id(&t.path, r.line, &r.cells[0]) else {
                continue;
            };
            let Some(letter) = number(&r.cells[1]).filter(|n| *n > 0) else {
                self.err(&t.path, r.line, format!("bad letter {:?}", r.cells[1]));
                continue;
            };
            if r.cells[2] != name {
                self.err(
                    &t.path,
                    r.line,
                    format!("category {:?} in the table for {name:?}", r.cells[2]),
                );
            }
            if !CONTEXTS.contains(&r.cells[3].as_str()) {
                self.err(&t.path, r.line, format!("unknown context {:?}", r.cells[3]));
            }
            if !matches!(r.cells[4].as_str(), "" | "excluded") {
                self.err(
                    &t.path,
                    r.line,
                    format!("unknown exception {:?}", r.cells[4]),
                );
            }
            self.usul.push(UsulRow {
                file: t.path.clone(),
                line: r.line,
                word,
                letter,
                category: r.cells[2].clone(),
                context: r.cells[3].clone(),
                exception: r.cells[4].clone(),
                prov: Provenance {
                    source: r.cells[5].clone(),
                    status: r.cells[6].clone(),
                    support: r.cells[7].clone(),
                    note: r.cells[8].clone(),
                },
            });
        }
    }

    fn load_adad(&mut self, t: &Table) {
        for r in &t.rows {
            let (Some(surah), Some(ayah)) = (number(&r.cells[0]), number(&r.cells[1])) else {
                self.err(&t.path, r.line, "bad surah or ayah number");
                continue;
            };
            let (Some(first), Some(last)) = (
                self.word_id(&t.path, r.line, &r.cells[2]),
                self.word_id(&t.path, r.line, &r.cells[3]),
            ) else {
                continue;
            };
            self.adad.push(AdadRow {
                file: t.path.clone(),
                line: r.line,
                surah,
                ayah,
                first,
                last,
                source: r.cells[4].clone(),
                status: r.cells[5].clone(),
            });
        }
    }

    fn load_lines(&mut self, t: &Table) {
        for r in &t.rows {
            let (Some(page), Some(line_no)) = (number(&r.cells[0]), number(&r.cells[1])) else {
                self.err(&t.path, r.line, "bad page or line number");
                continue;
            };
            let mut opt = |s: &str| {
                if s.is_empty() {
                    Some(None)
                } else {
                    self.word_id(&t.path, r.line, s).map(Some)
                }
            };
            let (Some(first), Some(last)) = (opt(&r.cells[3]), opt(&r.cells[4])) else {
                continue;
            };
            let centered = match r.cells[5].as_str() {
                "true" => true,
                "false" => false,
                other => {
                    self.err(
                        &t.path,
                        r.line,
                        format!("centered must be true or false, not {other:?}"),
                    );
                    false
                }
            };
            self.layout_lines.push(LayoutLine {
                file: t.path.clone(),
                line: r.line,
                page,
                line_no,
                kind: r.cells[2].clone(),
                first,
                last,
                centered,
            });
        }
    }

    /// Check the whole corpus: per-row errors found while loading, provenance, references
    /// between tables, order and uniqueness of keys, and approvals.
    pub fn check(&self) -> Report {
        let mut r = Report {
            errors: self.load_errors.clone(),
            ..Report::default()
        };
        let sources: BTreeSet<&str> = self.sources.iter().map(|s| s.id.as_str()).collect();
        let mut seen = BTreeSet::new();
        for s in &self.sources {
            if !seen.insert(s.id.as_str()) {
                r.errors.push(Diag::new(
                    SOURCES,
                    None,
                    format!("duplicate source id {:?}", s.id),
                ));
            }
            if !["truth", "check", "text"].contains(&s.role.as_str()) {
                r.errors.push(Diag::new(
                    SOURCES,
                    None,
                    format!("{}: unknown role {:?}", s.id, s.role),
                ));
            }
        }

        // Words: order and uniqueness, sources.
        let mut letters: BTreeMap<WordId, usize> = BTreeMap::new();
        let mut prev: Option<WordId> = None;
        for w in &self.words {
            if prev.is_some_and(|p| p >= w.id) {
                r.errors.push(Diag::new(
                    WORDS,
                    Some(w.line),
                    format!("{} out of order or duplicate", w.id),
                ));
            }
            prev = Some(w.id);
            check_sources(&mut r, &sources, WORDS, w.line, &w.source);
            match mushaf_text::clusterize(&w.text) {
                Ok(c) => {
                    letters.insert(w.id, c.len());
                }
                Err(e) => r
                    .errors
                    .push(Diag::new(WORDS, Some(w.line), format!("{}: {e}", w.id))),
            }
        }

        // Uṣūl rows.
        let mut keys: BTreeMap<(&str, WordId, u16), usize> = BTreeMap::new();
        let mut last: BTreeMap<&str, (WordId, u16)> = BTreeMap::new();
        for u in &self.usul {
            check_prov(&mut r, &sources, &u.file, u.line, &u.prov);
            match letters.get(&u.word) {
                None => r.errors.push(Diag::new(
                    &u.file,
                    Some(u.line),
                    format!("word {} is not in the word table", u.word),
                )),
                Some(n) if usize::from(u.letter) > *n => r.errors.push(Diag::new(
                    &u.file,
                    Some(u.line),
                    format!(
                        "letter {} past the end of {} ({n} clusters)",
                        u.letter, u.word
                    ),
                )),
                Some(_) => {}
            }
            let key = (u.file.as_str(), u.word, u.letter);
            if keys.insert(key, u.line).is_some() {
                r.errors.push(Diag::new(
                    &u.file,
                    Some(u.line),
                    format!("duplicate row {}:{}", u.word, u.letter),
                ));
            } else if last
                .get(u.file.as_str())
                .is_some_and(|p| *p > (u.word, u.letter))
            {
                r.errors
                    .push(Diag::new(&u.file, Some(u.line), "rows out of order"));
            }
            last.insert(u.file.as_str(), (u.word, u.letter));
        }

        // Ayah counting rows.
        for a in &self.adad {
            check_status(&mut r, &a.file, a.line, &a.status);
            check_sources(&mut r, &sources, &a.file, a.line, &a.source);
            for w in [a.first, a.last] {
                if !letters.contains_key(&w) {
                    r.errors.push(Diag::new(
                        &a.file,
                        Some(a.line),
                        format!("word {w} is not in the word table"),
                    ));
                }
            }
        }

        // Layout lines.
        for l in &self.layout_lines {
            for w in [l.first, l.last].into_iter().flatten() {
                if !letters.contains_key(&w) {
                    r.errors.push(Diag::new(
                        &l.file,
                        Some(l.line),
                        format!("word {w} is not in the word table"),
                    ));
                }
            }
        }

        // Approvals (D70).
        for a in &self.approvals {
            match self.hashes.get(&a.table) {
                None => r.errors.push(Diag::new(
                    APPROVALS,
                    None,
                    format!("approval for {} names no corpus file", a.table),
                )),
                Some(h) => {
                    let state = if *h == a.sha256 { "current" } else { "stale" };
                    r.approvals.insert(a.table.clone(), state.to_string());
                }
            }
            if !OWNER_STATUSES.contains(&a.status.as_str()) {
                r.errors.push(Diag::new(
                    APPROVALS,
                    None,
                    format!("{}: unknown status {:?}", a.table, a.status),
                ));
            }
        }
        r
    }
}

fn tsv_under<'a>(path: &'a str, dir: &str) -> Option<&'a str> {
    path.strip_prefix(dir)?
        .strip_suffix(".tsv")
        .filter(|n| !n.contains('/'))
}

fn check_status(r: &mut Report, file: &str, line: usize, status: &str) {
    if OWNER_STATUSES.contains(&status) {
        r.errors.push(Diag::new(
            file,
            Some(line),
            format!("status {status:?} is set only in data/approvals/ (rule 4)"),
        ));
    } else if !STATUSES.contains(&status) {
        r.errors.push(Diag::new(
            file,
            Some(line),
            format!("unknown status {status:?}"),
        ));
    }
}

fn check_sources(r: &mut Report, known: &BTreeSet<&str>, file: &str, line: usize, cell: &str) {
    if cell.is_empty() {
        r.errors.push(Diag::new(file, Some(line), "no source"));
        return;
    }
    for part in cell.split(" ; ") {
        match part.split_once(':') {
            None => r.errors.push(Diag::new(
                file,
                Some(line),
                format!("source {part:?} has no locator"),
            )),
            Some((id, loc)) => {
                if !known.contains(id) {
                    r.errors.push(Diag::new(
                        file,
                        Some(line),
                        format!("source {id:?} is not in {SOURCES}"),
                    ));
                }
                if loc.is_empty() {
                    r.errors.push(Diag::new(
                        file,
                        Some(line),
                        format!("source {id:?} has an empty locator"),
                    ));
                }
            }
        }
    }
}

fn check_prov(r: &mut Report, known: &BTreeSet<&str>, file: &str, line: usize, p: &Provenance) {
    check_sources(r, known, file, line, &p.source);
    check_status(r, file, line, &p.status);
    if !p.support.is_empty() && !SUPPORT.contains(&p.support.as_str()) {
        r.errors.push(Diag::new(
            file,
            Some(line),
            format!("unknown support {:?}", p.support),
        ));
    }
}
