//! M0.5 acceptance: every item of the KFGQPC Ḥafṣ word table decodes into letters with slotted
//! marks and encodes back byte for byte, and the KFGQPC Madinah mark order
//! (`corpus/dabt/kfgqpc-madinah.yaml`, derived from the same text) holds on every letter except
//! the ones that file lists. No expected values about the Qurʾān are written here (rule 1): the
//! table and the style file are the data, the test checks they agree.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mushaf_corpus::{Corpus, Files, parse_cp};
use mushaf_text::{Unit, decode, encode, in_order};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn corpus() -> Corpus {
    let mut f = Files::new();
    for rel in [
        "corpus/sources.yaml",
        "corpus/text/words.tsv",
        "corpus/dabt/kfgqpc-madinah.yaml",
    ] {
        f.insert(
            rel.into(),
            std::fs::read_to_string(repo().join(rel)).unwrap(),
        );
    }
    Corpus::load(&f).unwrap()
}

#[test]
fn every_word_round_trips_and_follows_the_mark_order() {
    let c = corpus();
    assert!(c.check().errors.is_empty(), "{:?}", c.check().errors);
    let style = &c.styles["corpus/dabt/kfgqpc-madinah.yaml"];
    let order = style.order();
    let listed: BTreeSet<&str> = style
        .mark_order
        .exceptions
        .iter()
        .map(|x| x.letter.as_str())
        .collect();
    let mut out_of_order = BTreeSet::new();
    let mut letters = 0usize;
    for w in &c.words {
        let units = decode(&w.text).unwrap_or_else(|e| panic!("{}: {e}", w.id));
        assert_eq!(encode(&units), w.text, "{}", w.id);
        for (n, u) in units.iter().enumerate() {
            if let Unit::Letter(l) = u {
                letters += 1;
                if !in_order(&l.marks, &order) {
                    out_of_order.insert(format!("{}:{}", w.id, n + 1));
                }
            }
        }
    }
    let listed: BTreeSet<String> = listed.into_iter().map(String::from).collect();
    assert_eq!(
        out_of_order, listed,
        "letters out of the derived order vs the listed exceptions"
    );
    assert_eq!(style.letters, Some(letters as u64));
    // Each exception records the marks as the text writes them.
    for x in &style.mark_order.exceptions {
        assert!(x.marks.iter().all(|m| parse_cp(m).is_some()));
    }
}

/// M1.2 acceptance: every letter of the word table decodes into features through the KFGQPC
/// Madinah sign rows and encodes back to the same marks, in the same order, byte for byte.
#[test]
fn every_word_round_trips_through_features() {
    use std::collections::BTreeMap;

    use mushaf_text::{features_of, marks_for};

    let c = corpus();
    let style = &c.styles["corpus/dabt/kfgqpc-madinah.yaml"];
    let rules = style.rules();
    assert_eq!(rules.len(), style.signs.len());
    let order = style.order();
    let exceptions: BTreeMap<&str, Vec<char>> = style
        .mark_order
        .exceptions
        .iter()
        .map(|x| {
            let marks = x.marks.iter().filter_map(|m| parse_cp(m)).collect();
            (x.letter.as_str(), marks)
        })
        .collect();
    let mut used: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = Vec::new();
    for w in &c.words {
        let word = w.id.to_string();
        let mut units = decode(&w.text).unwrap();
        for (n, u) in units.iter_mut().enumerate() {
            let Unit::Letter(l) = u else { continue };
            let lid = format!("{word}:{}", n + 1);
            let res = features_of(l, &word, &lid, &rules).and_then(|(f, ids)| {
                for id in ids {
                    *used.entry(id).or_default() += 1;
                }
                let x = exceptions.get(lid.as_str()).map(Vec::as_slice);
                marks_for(l.base, &f, &word, &lid, &rules, &order, x)
            });
            match res {
                Ok(m) => l.marks = m,
                Err(e) => failures.push(e.to_string()),
            }
        }
        if failures.len() > 20 {
            break;
        }
        assert_eq!(encode(&units), w.text, "{}", w.id);
    }
    assert!(
        failures.is_empty(),
        "{} failures: {failures:?}",
        failures.len()
    );
    // Every mark row explains at least one mark (sign rows for standalone signs excepted).
    let unused: Vec<&str> = style
        .signs
        .iter()
        .filter(|r| !used.contains_key(&r.id))
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(
        unused,
        ["sajdah_place", "hizb_start"],
        "rows that explain no mark"
    );
}
