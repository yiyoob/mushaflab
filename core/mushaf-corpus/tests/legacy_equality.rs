//! M1.4 (D73): the corpus, converted to the engine's input types, equals the legacy Ḥafṣ spine
//! and layout item for item and line for line. Both sides are tracked data; no expected value
//! about the Qurʾān is written here (rule 1).

use std::path::{Path, PathBuf};

use mushaf_corpus::{Corpus, legacy};
use mushaf_model::DataSet;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn both() -> (Corpus, DataSet) {
    let c = Corpus::load(&mushaf_corpus::read_tree(&repo()).unwrap()).unwrap();
    let ds = DataSet::from_files(&mushaf_model::read_tree(&repo().join("data")).unwrap()).unwrap();
    (c, ds)
}

#[test]
fn the_word_table_is_the_legacy_spine() {
    let (c, ds) = both();
    let new = legacy::spine(&c).unwrap();
    let old = &ds.spines[legacy::SPINE_ID];
    assert_eq!(
        (new.id.as_str(), new.script_id.as_str(), new.version),
        (old.id.as_str(), old.script_id.as_str(), old.version)
    );
    assert_eq!(new.items.len(), old.items.len());
    for (a, b) in new.items.iter().zip(&old.items) {
        assert_eq!(a, b, "{}", b.id);
    }
}

#[test]
fn every_corpus_layout_is_the_legacy_layout() {
    let (c, ds) = both();
    let new = legacy::layouts(&c).unwrap();
    assert!(!new.is_empty());
    for (id, a) in &new {
        let b = &ds.layouts[id];
        assert_eq!(a.lines.len(), b.lines.len(), "{id}");
        for (x, y) in a.lines.iter().zip(&b.lines) {
            assert_eq!(x, y, "{id} page {} line {}", y.page, y.line);
        }
        assert_eq!(a, b, "{id}");
    }
}

#[test]
fn applying_the_corpus_keeps_the_data_set_valid() {
    let (c, mut ds) = both();
    let before = ds.spines[legacy::SPINE_ID].clone();
    legacy::apply(&c, &mut ds).unwrap();
    assert_eq!(ds.spines[legacy::SPINE_ID], before);
}
