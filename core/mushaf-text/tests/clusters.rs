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

#[test]
fn every_mark_and_joiner_in_the_table_has_a_slot() {
    // Read the class column of the table itself, so a new mark added there must get a slot.
    let file = include_str!("../../../data/pipeline/codepoint_classes.tsv");
    for line in file.lines().skip(1) {
        let cols: Vec<&str> = line.split('\t').collect();
        if matches!(cols.get(1), Some(&"mark") | Some(&"joiner")) {
            let cp = u32::from_str_radix(cols[0].trim_start_matches("U+"), 16).unwrap();
            let c = char::from_u32(cp).unwrap();
            assert!(mushaf_text::slot_of(c).is_some(), "{} has no slot", cols[0]);
        }
    }
}

#[test]
fn decode_encode_and_place_on_synthetic_text() {
    use mushaf_text::{Unit, decode, encode, in_order, place};
    let text = "a\u{0651}\u{064E}b1";
    let units = decode(text).unwrap();
    assert_eq!(encode(&units), text);
    assert!(matches!(units[2], Unit::Standalone { .. }));
    let order = ['\u{0652}', '\u{0651}', '\u{064E}'];
    let Unit::Letter(l) = &units[0] else { panic!() };
    assert!(in_order(&l.marks, &order));
    let mut marks = vec!['\u{064E}'];
    place(&mut marks, '\u{0651}', &order).unwrap();
    assert_eq!(marks, ['\u{0651}', '\u{064E}']);
    assert!(place(&mut marks, '\u{0653}', &order).is_err());
}
