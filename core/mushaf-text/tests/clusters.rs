//! M0.3: clusters from the codepoint table (synthetic text only, CLAUDE.md rule 1).

use mushaf_text::{ClusterKind, class_count, clusterize};

#[test]
fn the_embedded_table_has_every_data_row() {
    // One class per data row of data/pipeline/codepoint_classes.tsv (header and blank lines
    // excluded); counted from the file itself, not from memory.
    let file = include_str!("../../../data/pipeline/codepoint_classes.tsv");
    let rows = file
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .count();
    assert_eq!(class_count(), rows);
}

#[test]
fn ascii_letters_are_bases_and_digits_standalone() {
    let c = clusterize("ab1").unwrap();
    let kinds: Vec<_> = c.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        [ClusterKind::Letter, ClusterKind::Letter, ClusterKind::Digit]
    );
    assert_eq!(c.iter().map(|c| c.text.as_str()).collect::<String>(), "ab1");
}

#[test]
fn a_mark_joins_the_base_before_it() {
    // U+064E is a mark in the table (class column), used here on a synthetic base.
    let c = clusterize("a\u{064E}b").unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].text, "a\u{064E}");
}

#[test]
fn unknown_codepoints_marks_without_base_and_spaces_are_errors() {
    assert!(clusterize("a?").unwrap_err().to_string().contains("U+003F"));
    assert!(
        clusterize("\u{064E}a")
            .unwrap_err()
            .to_string()
            .contains("no base")
    );
    assert!(clusterize("a b").unwrap_err().to_string().contains("space"));
}
