//! Letters and their signs (D72 level a): a lossless, convention-free view of a word.
//!
//! A letter is a base codepoint plus its marks in the order the text writes them. Each mark has a
//! slot, named after what the codepoint is in Unicode (a vowel sign, a tanwīn sign, a small
//! letter, a stop sign, …), never after what it means in a reading: meanings are rows of a
//! writing style with a citation (D72 level b, G-226). A mark without a slot is a hard failure.

use crate::{ClusterKind, Error, clusterize};

/// Where a mark sits in a letter, by what the codepoint is (D72).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slot {
    /// fatha, damma, kasra
    Vowel,
    /// fathatan, dammatan, kasratan
    Tanwin,
    Shadda,
    /// the sign of a vowelless letter, by whichever codepoint the convention uses
    Vowelless,
    Maddah,
    /// hamza above or below
    Hamza,
    /// superscript or subscript alef, small high yeh or noon, small high or low meem
    SmallLetter,
    /// the small high waqf ligatures and letters
    Stop,
    /// every other sign the texts use (small high seen, the zero signs, the stop marks used
    /// for imālah or tas-hīl, inverted damma, dot below, …) and the zero-width joiner
    Other,
}

/// The slot of a mark or joiner, or `None` if the codepoint is not one the texts use as a mark.
/// Grouping by Unicode character name; the codepoints are those of class `mark` and `joiner`
/// in `data/pipeline/codepoint_classes.tsv`.
pub fn slot_of(c: char) -> Option<Slot> {
    Some(match c {
        '\u{064E}' | '\u{064F}' | '\u{0650}' => Slot::Vowel,
        '\u{064B}' | '\u{064C}' | '\u{064D}' => Slot::Tanwin,
        '\u{0651}' => Slot::Shadda,
        '\u{0652}' | '\u{06E1}' => Slot::Vowelless,
        '\u{0653}' => Slot::Maddah,
        '\u{0654}' | '\u{0655}' => Slot::Hamza,
        '\u{0670}' | '\u{0656}' | '\u{06E7}' | '\u{06E8}' | '\u{06E2}' | '\u{06ED}' => {
            Slot::SmallLetter
        }
        '\u{06D6}' | '\u{06D7}' | '\u{06D8}' | '\u{06DA}' | '\u{06DB}' => Slot::Stop,
        '\u{0657}' | '\u{065C}' | '\u{065E}' | '\u{06DC}' | '\u{06DF}' | '\u{06E0}'
        | '\u{06E4}' | '\u{06EA}' | '\u{06EC}' | '\u{200D}' => Slot::Other,
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Letter {
    pub base: char,
    /// Marks in the order the text writes them.
    pub marks: Vec<char>,
}

/// One unit of a word: a letter, or a standalone sign, ayah number, digit or control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unit {
    Letter(Letter),
    Standalone { kind: ClusterKind, cp: char },
}

/// A word's text as units. Fails on a codepoint outside the table (via `clusterize`) and on a
/// mark without a slot.
pub fn decode(text: &str) -> Result<Vec<Unit>, Error> {
    clusterize(text)?
        .into_iter()
        .map(|c| {
            let mut chars = c.text.chars();
            let first = chars.next().ok_or_else(|| Error("empty cluster".into()))?;
            if c.kind != ClusterKind::Letter {
                return Ok(Unit::Standalone {
                    kind: c.kind,
                    cp: first,
                });
            }
            let marks: Vec<char> = chars.collect();
            if let Some(m) = marks.iter().find(|m| slot_of(**m).is_none()) {
                return Err(Error(format!("U+{:04X} has no slot (D72)", *m as u32)));
            }
            Ok(Unit::Letter(Letter { base: first, marks }))
        })
        .collect()
}

/// The text of units, exactly as `decode` read it.
pub fn encode(units: &[Unit]) -> String {
    let mut s = String::new();
    for u in units {
        match u {
            Unit::Letter(l) => {
                s.push(l.base);
                s.extend(&l.marks);
            }
            Unit::Standalone { cp, .. } => s.push(*cp),
        }
    }
    s
}

/// Whether `marks` follow a convention's mark order. Marks the order does not list never count
/// as in order.
pub fn in_order(marks: &[char], order: &[char]) -> bool {
    let rank: Option<Vec<usize>> = marks
        .iter()
        .map(|m| order.iter().position(|o| o == m))
        .collect();
    rank.is_some_and(|r| r.windows(2).all(|w| w[0] < w[1]))
}

/// Add a mark where the convention's order puts it (after every mark that comes before it).
/// Returns an error if the order does not list it.
pub fn place(marks: &mut Vec<char>, new: char, order: &[char]) -> Result<(), Error> {
    let rank = |c: char| order.iter().position(|o| *o == c);
    let r = rank(new).ok_or_else(|| {
        Error(format!(
            "U+{:04X} is not in the convention's mark order",
            new as u32
        ))
    })?;
    let at = marks
        .iter()
        .position(|m| rank(*m).is_some_and(|x| x > r))
        .unwrap_or(marks.len());
    marks.insert(at, new);
    Ok(())
}
