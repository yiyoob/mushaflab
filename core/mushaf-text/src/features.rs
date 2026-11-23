//! Features of a letter through a writing style's sign rules (D72 level b, M1.2).
//!
//! A rule says what one mark means in one context: its codepoint, the features it marks, and
//! optionally the bases it sits on, the words it applies to, and features the letter must or
//! must not also have. Rules come from a style table (`corpus/dabt/<style>.yaml`), each with a
//! citation; this module only applies them.
//!
//! - `features_of` reads a letter's marks into a feature set (decode).
//! - `marks_for` writes a feature set back as marks in the style (encode): a rule is used when
//!   all its features are wanted and its context holds; every wanted feature must come from
//!   exactly one rule. Marks are ordered by the style's mark order, or by the letter's listed
//!   exception.
//!
//! A mark no rule explains, two rules explaining one mark, a feature no rule writes, or a
//! feature two rules write are hard failures that name the letter.

use std::collections::BTreeSet;

use crate::{Error, Letter};

/// One sign rule of a writing style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub id: String,
    pub cp: char,
    pub features: Vec<String>,
    /// Bases the mark sits on; empty means any.
    pub bases: Vec<char>,
    /// Word ids (`surah:ayah:word`) the rule applies to; empty means every word.
    pub at: Vec<String>,
    /// Features the letter must also have (from rules without conditions).
    pub with: Vec<String>,
    /// Features the letter must not have (from rules without conditions).
    pub without: Vec<String>,
}

impl Rule {
    fn place_ok(&self, base: char, word: &str) -> bool {
        (self.bases.is_empty() || self.bases.contains(&base))
            && (self.at.is_empty() || self.at.iter().any(|w| w == word))
    }

    fn conditional(&self) -> bool {
        !self.with.is_empty() || !self.without.is_empty()
    }

    fn context_ok(&self, have: &BTreeSet<String>) -> bool {
        self.with.iter().all(|f| have.contains(f)) && !self.without.iter().any(|f| have.contains(f))
    }
}

/// The features of a letter, and the id of the rule that explains each mark, in mark order.
/// `letter_id` only names the letter in errors.
pub fn features_of(
    letter: &Letter,
    word: &str,
    letter_id: &str,
    rules: &[Rule],
) -> Result<(BTreeSet<String>, Vec<String>), Error> {
    let mut have = BTreeSet::new();
    let mut used: Vec<Option<&Rule>> = vec![None; letter.marks.len()];
    // Rules without conditions first; conditions read only their features.
    for pass in [false, true] {
        for (k, m) in letter.marks.iter().enumerate() {
            if used[k].is_some() {
                continue;
            }
            let found: Vec<&Rule> = rules
                .iter()
                .filter(|r| r.cp == *m && r.place_ok(letter.base, word))
                .filter(|r| r.conditional() == pass && (!pass || r.context_ok(&have)))
                .collect();
            match found.as_slice() {
                [] => {}
                [r] => {
                    used[k] = Some(r);
                    if !pass {
                        have.extend(r.features.iter().cloned());
                    }
                }
                more => {
                    let ids: Vec<&str> = more.iter().map(|r| r.id.as_str()).collect();
                    return Err(Error(format!(
                        "{letter_id}: U+{:04X} matches rules {ids:?}",
                        *m as u32
                    )));
                }
            }
        }
    }
    let mut ids = Vec::new();
    for (k, r) in used.into_iter().enumerate() {
        let r = r.ok_or_else(|| {
            Error(format!(
                "{letter_id}: no rule explains U+{:04X}",
                letter.marks[k] as u32
            ))
        })?;
        if r.conditional() {
            have.extend(r.features.iter().cloned());
        }
        ids.push(r.id.clone());
    }
    Ok((have, ids))
}

/// The marks that write `features` on `base` in word `word`, in the style's order, or in the
/// order of `exception` when the letter is a listed exception.
pub fn marks_for(
    base: char,
    features: &BTreeSet<String>,
    word: &str,
    letter_id: &str,
    rules: &[Rule],
    order: &[char],
    exception: Option<&[char]>,
) -> Result<Vec<char>, Error> {
    let chosen: Vec<&Rule> = rules
        .iter()
        .filter(|r| r.place_ok(base, word))
        .filter(|r| r.features.iter().all(|f| features.contains(f)))
        .filter(|r| r.context_ok(features))
        .collect();
    for f in features {
        let by: Vec<&str> = chosen
            .iter()
            .filter(|r| r.features.contains(f))
            .map(|r| r.id.as_str())
            .collect();
        match by.len() {
            1 => {}
            0 => return Err(Error(format!("{letter_id}: no rule writes feature {f:?}"))),
            _ => {
                return Err(Error(format!(
                    "{letter_id}: feature {f:?} is written by rules {by:?}"
                )));
            }
        }
    }
    let mut marks: Vec<char> = chosen.iter().map(|r| r.cp).collect();
    match exception {
        Some(x) => {
            let mut want = x.to_vec();
            let mut have = marks.clone();
            want.sort_unstable();
            have.sort_unstable();
            if want != have {
                return Err(Error(format!(
                    "{letter_id}: listed exception marks differ from the features' marks"
                )));
            }
            marks = x.to_vec();
        }
        None => {
            let rank = |c: &char| order.iter().position(|o| o == c).unwrap_or(usize::MAX);
            marks.sort_by_key(rank);
        }
    }
    Ok(marks)
}
