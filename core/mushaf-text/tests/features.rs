//! M1.2: features through sign rules (synthetic rules and letters, CLAUDE.md rule 1).

use std::collections::BTreeSet;

use mushaf_text::{Letter, Rule, features_of, marks_for};

// Codepoints are marks of the table used as placeholders; the feature names are invented.
const A: char = '\u{064E}';
const B: char = '\u{0651}';
const C: char = '\u{06E2}';
const D: char = '\u{06ED}';
const E: char = '\u{06DC}';

fn rule(id: &str, cp: char, f: &[&str]) -> Rule {
    Rule {
        id: id.into(),
        cp,
        features: f.iter().map(|s| (*s).to_string()).collect(),
        bases: vec![],
        at: vec![],
        with: vec![],
        without: vec![],
    }
}

fn rules() -> Vec<Rule> {
    let mut low = rule("low", D, &["fx"]);
    low.with = vec!["fa".into()];
    let mut high = rule("high", C, &["fx"]);
    high.without = vec!["fa".into()];
    let mut here = rule("here", E, &["fe1"]);
    here.at = vec!["1:1:1".into()];
    here.bases = vec!['q'];
    let mut there = rule("there", E, &["fe2"]);
    there.at = vec!["1:1:2".into()];
    vec![
        rule("a", A, &["fa"]),
        rule("b", B, &["fb", "fb2"]),
        low,
        high,
        here,
        there,
    ]
}

fn set(f: &[&str]) -> BTreeSet<String> {
    f.iter().map(|s| (*s).to_string()).collect()
}

fn round_trip(base: char, marks: &[char], word: &str) -> BTreeSet<String> {
    let l = Letter {
        base,
        marks: marks.to_vec(),
    };
    let (f, _) = features_of(&l, word, "t", &rules()).unwrap();
    let order = [B, A, C, D, E];
    let back = marks_for(base, &f, word, "t", &rules(), &order, None).unwrap();
    assert_eq!(back, marks, "{f:?}");
    f
}

#[test]
fn features_decode_and_encode_back() {
    assert_eq!(round_trip('q', &[B, A], "1:1:1"), set(&["fa", "fb", "fb2"]));
    assert_eq!(round_trip('q', &[A, D], "1:1:1"), set(&["fa", "fx"]));
    assert_eq!(round_trip('q', &[C], "1:1:1"), set(&["fx"]));
    assert_eq!(round_trip('q', &[E], "1:1:1"), set(&["fe1"]));
    assert_eq!(round_trip('r', &[E], "1:1:2"), set(&["fe2"]));
}

#[test]
fn unexplained_marks_and_unwritable_features_fail() {
    let l = Letter {
        base: 'r',
        marks: vec![E],
    };
    // Rule "here" needs base q, "there" needs word 1:1:2.
    let e = features_of(&l, "1:1:1", "t", &rules()).unwrap_err();
    assert!(e.0.contains("no rule explains U+06DC"), "{e}");
    let e = marks_for('q', &set(&["nope"]), "1:1:1", "t", &rules(), &[], None).unwrap_err();
    assert!(e.0.contains("nope"), "{e}");
}

#[test]
fn ambiguous_rules_fail() {
    let mut r = rules();
    r.push(rule("a2", A, &["fa"]));
    let l = Letter {
        base: 'q',
        marks: vec![A],
    };
    assert!(
        features_of(&l, "1:1:1", "t", &r)
            .unwrap_err()
            .0
            .contains("matches rules")
    );
    assert!(
        marks_for('q', &set(&["fa"]), "1:1:1", "t", &r, &[], None)
            .unwrap_err()
            .0
            .contains("written by rules")
    );
}

#[test]
fn a_listed_exception_keeps_its_order() {
    let f = set(&["fa", "fb", "fb2"]);
    let order = [B, A];
    let m = marks_for('q', &f, "1:1:1", "t", &rules(), &order, Some(&[A, B])).unwrap();
    assert_eq!(m, [A, B]);
}
