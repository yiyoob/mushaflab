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

/// A writing style (`corpus/dabt/<id>.yaml`, D72).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub id: String,
    pub title: String,
    /// Letters the order was derived from (informational).
    #[serde(default)]
    pub letters: Option<u64>,
    pub mark_order: MarkOrder,
    /// Sign meanings (D72 level b); each row cited (G-226). Empty until rows are added.
    #[serde(default)]
    pub signs: Vec<SignRow>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkOrder {
    pub source: String,
    pub status: String,
    #[serde(default)]
    pub note: String,
    pub order: Vec<String>,
    #[serde(default)]
    pub exceptions: Vec<OrderException>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderException {
    pub letter: String,
    pub marks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignRow {
    /// Unique within the style.
    pub id: String,
    pub codepoint: String,
    pub features: Vec<String>,
    /// Bases (`U+XXXX`) the mark sits on; empty means any.
    #[serde(default)]
    pub bases: Vec<String>,
    /// Word ids the row applies to; empty means every word.
    #[serde(default)]
    pub at: Vec<String>,
    /// Features the letter must also have / must not have (D72 level b, M1.2).
    #[serde(default)]
    pub with: Vec<String>,
    #[serde(default)]
    pub without: Vec<String>,
    /// Words the cited page gives as examples; each must carry the codepoint.
    #[serde(default)]
    pub evidence: Vec<String>,
    pub source: String,
    pub status: String,
    #[serde(default)]
    pub note: String,
}

/// `U+XXXX` to a char.
pub fn parse_cp(s: &str) -> Option<char> {
    s.strip_prefix("U+")
        .filter(|h| (4..=6).contains(&h.len()))
        .and_then(|h| u32::from_str_radix(h, 16).ok())
        .and_then(char::from_u32)
}

impl Style {
    /// The sign rows as `mushaf_text` rules (rows with unreadable codepoints are skipped; they
    /// are reported by `Corpus::check`).
    pub fn rules(&self) -> Vec<mushaf_text::Rule> {
        self.signs
            .iter()
            .filter_map(|r| {
                Some(mushaf_text::Rule {
                    id: r.id.clone(),
                    cp: parse_cp(&r.codepoint)?,
                    features: r.features.clone(),
                    bases: r.bases.iter().filter_map(|b| parse_cp(b)).collect(),
                    at: r.at.clone(),
                    with: r.with.clone(),
                    without: r.without.clone(),
                })
            })
            .collect()
    }

    /// The mark order as chars (unparsable entries are reported by `Corpus::check`).
    pub fn order(&self) -> Vec<char> {
        self.mark_order
            .order
            .iter()
            .filter_map(|c| parse_cp(c))
            .collect()
    }
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

/// A layout's description (`corpus/layouts/<id>/layout.yaml`, docs/formats.md).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub layout_id: String,
    pub script_id: String,
    pub edition: String,
    pub pages: u16,
    pub lines_per_page: u16,
    pub writing_style: String,
    pub font: String,
    #[serde(default)]
    pub render_profile: Option<String>,
    pub sources: Vec<LayoutSource>,
    /// Owner corrections already applied to the lines, kept with their evidence.
    #[serde(default)]
    pub corrections: Vec<LayoutCorrection>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutSource {
    /// A source id registered in `corpus/sources.yaml`.
    pub source: String,
    /// The `MANIFEST.json` pins the lines were read from.
    pub pins: Vec<String>,
    /// The tracked file the lines were converted from, if any.
    #[serde(default)]
    pub via: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutCorrection {
    pub page: u16,
    pub line: u16,
    pub end: String,
    pub was: String,
    pub set: String,
    pub evidence: String,
    #[serde(default)]
    pub via: Option<String>,
}

/// Everything loaded from `corpus/` (plus the approvals file).
#[derive(Debug, Default)]
pub struct Corpus {
    pub sources: Vec<Source>,
    pub words: Vec<Word>,
    pub usul: Vec<UsulRow>,
    pub adad: Vec<AdadRow>,
    pub layout_lines: Vec<LayoutLine>,
    /// Layout descriptions by path (`corpus/layouts/<id>/layout.yaml`).
    pub layouts: BTreeMap<String, Layout>,
    /// Writing styles by path.
    pub styles: BTreeMap<String, Style>,
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
const ROLES: [&str; 4] = ["truth", "check", "text", "layout"];
const LINE_TYPES: [&str; 3] = ["text", "surah_name", "basmalah"];
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
            if path.starts_with("corpus/dabt/") && path.ends_with(".yaml") {
                match mushaf_model::from_yaml::<Style>(text) {
                    Ok(st) => {
                        c.styles.insert(path.clone(), st);
                    }
                    Err(e) => fatal.push(Diag::new(path, None, e.to_string())),
                }
                continue;
            }
            if path.starts_with("corpus/layouts/") && path.ends_with("/layout.yaml") {
                match mushaf_model::from_yaml::<Layout>(text) {
                    Ok(l) => {
                        c.layouts.insert(path.clone(), l);
                    }
                    Err(e) => fatal.push(Diag::new(path, None, e.to_string())),
                }
                continue;
            }
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
            if !ROLES.contains(&s.role.as_str()) {
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
        let kinds: BTreeMap<WordId, &str> =
            self.words.iter().map(|w| (w.id, w.kind.as_str())).collect();
        self.check_adad(&mut r, &sources, &kinds);

        // Layouts.
        let order: Vec<(WordId, &str)> =
            self.words.iter().map(|w| (w.id, w.kind.as_str())).collect();
        self.check_layouts(&mut r, &sources, &order);

        let texts: BTreeMap<WordId, &str> =
            self.words.iter().map(|w| (w.id, w.text.as_str())).collect();

        // Writing styles (D72).
        for (path, st) in &self.styles {
            let mo = &st.mark_order;
            check_sources(&mut r, &sources, path, 0, &mo.source);
            check_status(&mut r, path, 0, &mo.status);
            let order = st.order();
            if order.len() != mo.order.len() {
                r.errors.push(Diag::new(
                    path,
                    None,
                    "mark_order has an unreadable codepoint",
                ));
            }
            for x in &mo.exceptions {
                let lid = x.letter.rsplit_once(':');
                let ok = lid.and_then(|(w, n)| Some((WordId::parse(w)?, number(n)?)));
                match ok {
                    Some((w, n))
                        if letters
                            .get(&w)
                            .is_some_and(|k| usize::from(n) <= *k && n > 0) => {}
                    _ => r.errors.push(Diag::new(
                        path,
                        None,
                        format!("exception {:?} names no letter", x.letter),
                    )),
                }
            }
            let mut ids = BTreeSet::new();
            for row in &st.signs {
                let here = |msg: String| Diag::new(path, None, format!("sign {}: {msg}", row.id));
                check_sources(&mut r, &sources, path, 0, &row.source);
                check_status(&mut r, path, 0, &row.status);
                if !ids.insert(row.id.as_str()) {
                    r.errors.push(here("duplicate id".into()));
                }
                let cp = parse_cp(&row.codepoint);
                if cp.is_none() {
                    r.errors
                        .push(here(format!("bad codepoint {:?}", row.codepoint)));
                }
                if row.features.is_empty() {
                    r.errors.push(here("no features".into()));
                }
                for b in &row.bases {
                    if parse_cp(b).is_none() {
                        r.errors.push(here(format!("bad base {b:?}")));
                    }
                }
                let bases: Vec<char> = row.bases.iter().filter_map(|b| parse_cp(b)).collect();
                for w in row.at.iter().chain(&row.evidence) {
                    let Some(id) = WordId::parse(w).filter(|id| letters.contains_key(id)) else {
                        r.errors
                            .push(here(format!("word {w:?} is not in the word table")));
                        continue;
                    };
                    // Evidence words must carry the codepoint (on one of the bases, if given).
                    if !row.evidence.contains(w) {
                        continue;
                    }
                    let text = texts.get(&id).copied().unwrap_or_default();
                    let ok = cp.is_some_and(|cp| carries(text, cp, &bases));
                    if !ok {
                        r.errors.push(here(format!(
                            "evidence {w} does not carry {}",
                            row.codepoint
                        )));
                    }
                    if !row.at.is_empty() && !row.at.contains(w) {
                        r.errors
                            .push(here(format!("evidence {w} is outside the row's words")));
                    }
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

impl Corpus {
    fn check_adad(&self, r: &mut Report, sources: &BTreeSet<&str>, kinds: &BTreeMap<WordId, &str>) {
        let mut prev: BTreeMap<&str, &AdadRow> = BTreeMap::new();
        for a in &self.adad {
            let here = |msg: String| Diag::new(&a.file, Some(a.line), msg);
            check_status(r, &a.file, a.line, &a.status);
            check_sources(r, sources, &a.file, a.line, &a.source);
            for w in [a.first, a.last] {
                match kinds.get(&w) {
                    None => r
                        .errors
                        .push(here(format!("word {w} is not in the word table"))),
                    Some(&"word") => {}
                    Some(k) => r.errors.push(here(format!("{w} is not a word item ({k})"))),
                }
            }
            if a.last < a.first {
                r.errors.push(here(format!(
                    "{}:{} ends before it starts ({} > {})",
                    a.surah, a.ayah, a.first, a.last
                )));
            }
            match prev.get(a.file.as_str()) {
                Some(p) if (p.surah, p.ayah) >= (a.surah, a.ayah) => {
                    r.errors.push(here(format!(
                        "{}:{} out of order or duplicate",
                        a.surah, a.ayah
                    )));
                }
                p => {
                    let want = match p {
                        Some(p) if p.surah == a.surah => p.ayah + 1,
                        _ => 1,
                    };
                    if a.ayah != want {
                        r.errors.push(here(format!(
                            "ayahs not contiguous: {}:{} where {}:{want} was expected",
                            a.surah, a.ayah, a.surah
                        )));
                    }
                    if let Some(p) = p.filter(|p| a.first <= p.last) {
                        r.errors.push(here(format!(
                            "{}:{} overlaps {}:{} ({} <= {})",
                            a.surah, a.ayah, p.surah, p.ayah, a.first, p.last
                        )));
                    }
                }
            }
            prev.insert(a.file.as_str(), a);
        }
    }

    fn check_layouts(&self, r: &mut Report, sources: &BTreeSet<&str>, order: &[(WordId, &str)]) {
        let index: BTreeMap<WordId, usize> = order
            .iter()
            .enumerate()
            .map(|(i, (w, _))| (*w, i))
            .collect();
        let mut by_dir: BTreeMap<&str, Vec<&LayoutLine>> = BTreeMap::new();
        for l in &self.layout_lines {
            let dir = l.file.strip_suffix("lines.tsv").unwrap_or(&l.file);
            by_dir.entry(dir).or_default().push(l);
        }
        for (path, lay) in &self.layouts {
            let dir = path.strip_suffix("layout.yaml").unwrap_or(path);
            let name = dir
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or_default();
            if lay.layout_id != name {
                r.errors.push(Diag::new(
                    path,
                    None,
                    format!(
                        "layout_id {:?} differs from its directory {name:?}",
                        lay.layout_id
                    ),
                ));
            }
            for s in &lay.sources {
                if !sources.contains(s.source.as_str()) {
                    r.errors.push(Diag::new(
                        path,
                        None,
                        format!("source {:?} is not in {SOURCES}", s.source),
                    ));
                }
                check_status(r, path, 0, &s.status);
            }
            if !by_dir.contains_key(dir) {
                r.errors
                    .push(Diag::new(path, None, format!("no {dir}lines.tsv")));
            }
        }
        for (dir, lines) in &by_dir {
            let yaml = format!("{dir}layout.yaml");
            let Some(lay) = self.layouts.get(&yaml) else {
                r.errors.push(Diag::new(
                    &lines[0].file,
                    None,
                    format!("no {yaml} for this lines table"),
                ));
                continue;
            };
            check_lines(r, lay, lines, order, &index);
        }
    }
}

/// One layout's lines: numbering, types, and coverage of the word table in order.
fn check_lines(
    r: &mut Report,
    lay: &Layout,
    lines: &[&LayoutLine],
    order: &[(WordId, &str)],
    index: &BTreeMap<WordId, usize>,
) {
    let mut prev: Option<(u16, u16)> = None;
    let mut pages = BTreeSet::new();
    // Next item of the word table a text line must start at.
    let mut next = 0usize;
    for (k, l) in lines.iter().enumerate() {
        let here = |msg: String| Diag::new(&l.file, Some(l.line), msg);
        let key = (l.page, l.line_no);
        if prev.is_some_and(|p| p >= key) {
            r.errors.push(here(format!(
                "page {} line {} out of order",
                l.page, l.line_no
            )));
        }
        prev = Some(key);
        pages.insert(l.page);
        if l.page == 0 || l.page > lay.pages {
            r.errors.push(here(format!(
                "page {} outside 1..={} (pages)",
                l.page, lay.pages
            )));
        }
        if l.line_no == 0 || l.line_no > lay.lines_per_page {
            r.errors.push(here(format!(
                "line {} outside 1..={} (lines_per_page)",
                l.line_no, lay.lines_per_page
            )));
        }
        if !LINE_TYPES.contains(&l.kind.as_str()) {
            r.errors
                .push(here(format!("unknown line type {:?}", l.kind)));
            continue;
        }
        if l.kind != "text" {
            if l.first.is_some() || l.last.is_some() {
                r.errors
                    .push(here(format!("a {} line has no word range", l.kind)));
            }
            if l.kind == "surah_name" {
                let starts = lines[k + 1..]
                    .iter()
                    .take_while(|m| m.page == l.page && m.kind != "surah_name")
                    .find(|m| m.kind == "text")
                    .and_then(|m| m.first);
                if !starts.is_some_and(|w| w.ayah == 1 && w.word == 1) {
                    r.errors.push(here(format!(
                        "surah_name on page {} line {} is not followed on its page by a line \
                         starting a surah",
                        l.page, l.line_no
                    )));
                }
            }
            continue;
        }
        let (Some(first), Some(last)) = (l.first, l.last) else {
            r.errors.push(here("a text line needs a word range".into()));
            continue;
        };
        let (Some(&i), Some(&j)) = (index.get(&first), index.get(&last)) else {
            for w in [first, last] {
                if !index.contains_key(&w) {
                    r.errors
                        .push(here(format!("word {w} is not in the word table")));
                }
            }
            continue;
        };
        // A ḥizb sign belongs with the word after it, so a line may start just after one.
        while next < order.len() && order[next].1 == "hizb" && next < i {
            next += 1;
        }
        if i != next {
            let want = order[next.min(order.len())..]
                .iter()
                .find(|(_, k)| *k != "hizb")
                .map_or("the end".into(), |w| w.0.to_string());
            r.errors.push(here(format!(
                "line starts at {first}, expected {want} (gap or overlap)"
            )));
        }
        if j < i {
            r.errors.push(here(format!(
                "line ends before it starts ({first} > {last})"
            )));
        }
        next = next.max(j + 1);
    }
    if let Some((w, _)) = order[next.min(order.len())..]
        .iter()
        .find(|(_, k)| *k != "hizb")
    {
        r.errors.push(Diag::new(
            &lines[lines.len() - 1].file,
            None,
            format!("words from {w} on are on no line"),
        ));
    }
    if let Some(&last) = pages.last()
        && (pages.len() != usize::from(last) || last != lay.pages)
    {
        r.errors.push(Diag::new(
            &lines[0].file,
            None,
            format!(
                "pages are not 1..={} without gaps (pages in layout.yaml)",
                lay.pages
            ),
        ));
    }
}

/// Whether `text` has `cp` as a standalone sign or as a mark on one of `bases` (any base when
/// empty).
fn carries(text: &str, cp: char, bases: &[char]) -> bool {
    let Ok(units) = mushaf_text::decode(text) else {
        return false;
    };
    units.iter().any(|u| match u {
        mushaf_text::Unit::Letter(l) => {
            l.marks.contains(&cp) && (bases.is_empty() || bases.contains(&l.base))
        }
        mushaf_text::Unit::Standalone { cp: c, .. } => *c == cp,
    })
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
            at(line),
            format!("status {status:?} is set only in data/approvals/ (rule 4)"),
        ));
    } else if !STATUSES.contains(&status) {
        r.errors.push(Diag::new(
            file,
            at(line),
            format!("unknown status {status:?}"),
        ));
    }
}

fn check_sources(r: &mut Report, known: &BTreeSet<&str>, file: &str, line: usize, cell: &str) {
    if cell.is_empty() {
        r.errors.push(Diag::new(file, at(line), "no source"));
        return;
    }
    for part in cell.split(" ; ") {
        match part.split_once(':') {
            None => r.errors.push(Diag::new(
                file,
                at(line),
                format!("source {part:?} has no locator"),
            )),
            Some((id, loc)) => {
                if !known.contains(id) {
                    r.errors.push(Diag::new(
                        file,
                        at(line),
                        format!("source {id:?} is not in {SOURCES}"),
                    ));
                }
                if loc.is_empty() {
                    r.errors.push(Diag::new(
                        file,
                        at(line),
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
            at(line),
            format!("unknown support {:?}", p.support),
        ));
    }
}

/// A 1-based line, or none for a whole-file fact (YAML files, line 0).
fn at(line: usize) -> Option<usize> {
    (line > 0).then_some(line)
}
