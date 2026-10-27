//! Maghribi -> Hijazi writing conventions for a resolved Maghribi riwayah (D46).
//!
//! Applied at render time only, to show Warsh or Qalun in the Hijazi style: the resolved text,
//! the spine and the patches are unchanged. Each convention is an owner decision
//! (`data/approvals/style-conventions.yaml`, 2026-10-26). Per cluster, the aligned Hafs
//! cluster (same id in the spine) decides:
//!
//! - when the two differ only in writing (sukun, alif wasla, final ya, the hamza form
//!   U+0627 U+06DF), the cluster is written as Hafs writes it;
//! - otherwise (a reading difference) the riwayah's own form is kept, with its sukun sign
//!   U+0652 taken from Hafs (U+06E1 or U+0652), or, where Hafs has neither, U+0652 on alif
//!   and U+06E1 on any other letter;
//! - a word with the same letters and the same marks as Hafs, stored on a neighbouring
//!   letter, is written as Hafs writes it;
//! - pause signs are always the riwayah's own (owner: "warsh and qalun should show their own
//!   pause signs").
//!
//! Cluster count and ids never change, so line mapping and layout are unaffected.

use mushaf_model::{Cps, ItemKind, Spine};

use crate::Resolved;

const SUKUN: char = '\u{06E1}';
const ROUND: char = '\u{0652}';
const ALIF: char = '\u{0627}';
const WASLA: char = '\u{0671}';
const YEH_BARREE: char = '\u{06D2}';
const VOWELS: [char; 3] = ['\u{064E}', '\u{064F}', '\u{0650}'];
const WASLA_MARKS: [char; 3] = ['\u{06EC}', '\u{06EA}', '\u{06DF}'];

fn is_pause(c: char) -> bool {
    ('\u{06D6}'..='\u{06DB}').contains(&c)
}

fn strip_pause(t: &[char]) -> (Vec<char>, Vec<char>) {
    t.iter().copied().partition(|c| !is_pause(*c))
}

/// Whether `h` (Hafs) and `w` (Maghribi), without pause signs, differ only in writing.
fn writing_only(h: &[char], w: &[char]) -> bool {
    if h == w {
        return true;
    }
    if h.first() == Some(&WASLA) && w.first() == Some(&ALIF) {
        let extra_ok = w[1..]
            .iter()
            .all(|c| h[1..].contains(c) || VOWELS.contains(c) || WASLA_MARKS.contains(c));
        return extra_ok && h[1..].iter().all(|c| w[1..].contains(c));
    }
    if matches!(h.first(), Some('\u{064A}' | '\u{0649}')) && w.first() == Some(&YEH_BARREE) {
        return h[1..] == w[1..];
    }
    if h == ['\u{0623}', '\u{064F}'] && w == [ALIF, '\u{06DF}'] {
        return true;
    }
    h.contains(&SUKUN)
        && h.iter()
            .map(|c| if *c == SUKUN { ROUND } else { *c })
            .eq(w.iter().copied())
}

/// The riwayah's own cluster with its sukun sign in Hijazi form.
fn sukun_only(w: &[char], h: Option<&[char]>) -> Vec<char> {
    let to = match h {
        Some(h) if h.contains(&SUKUN) => SUKUN,
        Some(h) if h.contains(&ROUND) => ROUND,
        _ if w.first() == Some(&ALIF) => ROUND,
        _ => SUKUN,
    };
    w.iter()
        .map(|c| if *c == ROUND { to } else { *c })
        .collect()
}

fn letters_and_marks(cl: &[Vec<char>]) -> (Vec<char>, Vec<char>) {
    let letters = cl.iter().filter_map(|c| c.first().copied()).collect();
    let mut marks: Vec<char> = cl.iter().flat_map(|c| c.iter().skip(1).copied()).collect();
    marks.sort_unstable();
    (letters, marks)
}

/// The resolved text of a Maghribi riwayah written in Hijazi conventions (D46).
pub fn maghribi_to_hijazi(resolved: &Resolved, spine: &Spine) -> Resolved {
    let mut out = resolved.clone();
    for item in out.items.iter_mut().filter(|i| i.kind == ItemKind::Word) {
        let hafs: Vec<Option<Vec<char>>> = item
            .clusters
            .iter()
            .map(|c| spine.cluster(&c.id).map(|h| strip_pause(&h.cps.0).0))
            .collect();
        let mut pauses = Vec::new();
        let mut conv: Vec<Vec<char>> = Vec::new();
        for (c, h) in item.clusters.iter().zip(&hafs) {
            let (w, p) = strip_pause(&c.cps.0);
            pauses.push(p);
            conv.push(match h {
                Some(h) if writing_only(h, &w) => h.clone(),
                h => sukun_only(&w, h.as_deref()),
            });
        }
        if let Some(h) = hafs.iter().cloned().collect::<Option<Vec<Vec<char>>>>()
            && letters_and_marks(&conv) == letters_and_marks(&h)
        {
            conv = h;
        }
        for ((c, mut new), p) in item.clusters.iter_mut().zip(conv).zip(pauses) {
            new.extend(p);
            c.cps = Cps(new);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn writing_conventions() {
        let lam = '\u{0644}';
        assert!(writing_only(&[lam, SUKUN], &[lam, ROUND]));
        assert!(!writing_only(&[lam, '\u{064E}'], &[lam, ROUND]));
        assert!(writing_only(&[WASLA], &[ALIF, '\u{064E}', '\u{06EC}']));
        assert!(writing_only(
            &['\u{064A}', '\u{064E}'],
            &[YEH_BARREE, '\u{064E}']
        ));
        assert!(writing_only(&['\u{0623}', '\u{064F}'], &[ALIF, '\u{06DF}']));
        assert!(!writing_only(&['\u{0623}', '\u{064E}'], &[ALIF]));
    }

    #[test]
    fn sukun_follows_hafs_then_the_letter() {
        let lam = '\u{0644}';
        assert_eq!(sukun_only(&[lam, ROUND], Some(&[lam, SUKUN])), [lam, SUKUN]);
        assert_eq!(
            sukun_only(&[ALIF, ROUND], Some(&[ALIF, ROUND])),
            [ALIF, ROUND]
        );
        assert_eq!(sukun_only(&[ALIF, ROUND], None), [ALIF, ROUND]);
        assert_eq!(sukun_only(&[lam, ROUND], Some(&[lam])), [lam, SUKUN]);
    }

    #[test]
    fn pause_signs_are_split_off() {
        let (t, p) = strip_pause(&w("\u{0644}\u{064E}\u{06D6}"));
        assert_eq!((t, p), (w("\u{0644}\u{064E}"), w("\u{06D6}")));
    }

    #[test]
    fn same_letters_and_marks_shifted_are_hafs() {
        let a = vec![w("\u{0628}\u{064E}\u{0651}"), w("\u{062A}")];
        let b = vec![w("\u{0628}\u{064E}"), w("\u{062A}\u{0651}")];
        assert_eq!(letters_and_marks(&a), letters_and_marks(&b));
    }
}
