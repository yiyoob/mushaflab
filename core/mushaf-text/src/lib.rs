//! Letters, signs and features of the Qurʾānic text (D72).
//!
//! M0.3: codepoint classes and clusters, ported from the legacy `data/pipeline/clusterize.py`
//! (same rules). A cluster is one base codepoint plus the marks and joiners that follow it, or a
//! standalone sign, ayah number, digit or control. A mark with no base, a space inside an item,
//! or a codepoint missing from the table is an error. Classes come only from the explicit table
//! `data/pipeline/codepoint_classes.tsv` (CLAUDE.md conventions), embedded at build time.
//!
//! ASCII letters and digits are accepted as bases and digits so synthetic fixtures (Latin letters
//! only, rule 1) can be clustered. The real text never contains them: its import checks every
//! codepoint against the table.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

const TABLE: &str = include_str!("../../../data/pipeline/codepoint_classes.tsv");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Class {
    Space,
    Base,
    Mark,
    Sign,
    AyahNumber,
    Digit,
    Joiner,
    Control,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ClusterKind {
    Letter,
    Sign,
    AyahNumber,
    Digit,
    Control,
}

impl ClusterKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ClusterKind::Letter => "letter",
            ClusterKind::Sign => "sign",
            ClusterKind::AyahNumber => "ayah_number",
            ClusterKind::Digit => "digit",
            ClusterKind::Control => "control",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cluster {
    pub kind: ClusterKind,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn parse_class(s: &str) -> Option<Class> {
    Some(match s {
        "space" => Class::Space,
        "base" => Class::Base,
        "mark" => Class::Mark,
        "sign" => Class::Sign,
        "ayah_number" => Class::AyahNumber,
        "digit" => Class::Digit,
        "joiner" => Class::Joiner,
        "control" => Class::Control,
        _ => return None,
    })
}

fn table() -> &'static BTreeMap<char, Class> {
    static T: OnceLock<BTreeMap<char, Class>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = BTreeMap::new();
        for (n, line) in TABLE.lines().enumerate().skip(1) {
            if line.trim().is_empty() {
                continue;
            }
            let mut cols = line.split('\t');
            let cp = cols.next().and_then(|c| c.strip_prefix("U+"));
            let class = cols.next().and_then(parse_class);
            let c = cp
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .and_then(char::from_u32);
            match (c, class) {
                (Some(c), Some(class)) => {
                    m.insert(c, class);
                }
                // The table is tested by the pipeline; a bad row here is a build defect.
                _ => panic!("codepoint_classes.tsv line {}: unreadable row", n + 1),
            }
        }
        m
    })
}

/// Number of codepoints in the embedded table.
pub fn class_count() -> usize {
    table().len()
}

/// The class of a codepoint: from the table, else ASCII letters (base) and digits (digit) for
/// synthetic fixtures, else `None`.
pub fn class_of(c: char) -> Option<Class> {
    table().get(&c).copied().or(if c.is_ascii_alphabetic() {
        Some(Class::Base)
    } else if c.is_ascii_digit() {
        Some(Class::Digit)
    } else {
        None
    })
}

/// Split one item's text into clusters.
pub fn clusterize(text: &str) -> Result<Vec<Cluster>, Error> {
    let mut out: Vec<Cluster> = Vec::new();
    let mut open_base = false;
    for c in text.chars() {
        let class = class_of(c)
            .ok_or_else(|| Error(format!("U+{:04X} is not in the codepoint table", c as u32)))?;
        let kind = match class {
            Class::Space => {
                return Err(Error(format!("space U+{:04X} inside an item", c as u32)));
            }
            Class::Mark | Class::Joiner => {
                match out.last_mut() {
                    Some(last) if open_base => last.text.push(c),
                    _ => return Err(Error(format!("U+{:04X} has no base", c as u32))),
                }
                continue;
            }
            Class::Base => ClusterKind::Letter,
            Class::Sign => ClusterKind::Sign,
            Class::AyahNumber => ClusterKind::AyahNumber,
            Class::Digit => ClusterKind::Digit,
            Class::Control => ClusterKind::Control,
        };
        open_base = class == Class::Base;
        out.push(Cluster {
            kind,
            text: c.to_string(),
        });
    }
    Ok(out)
}
