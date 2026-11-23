//! M0.3: the corpus validator on synthetic files (Latin letters only, CLAUDE.md rule 1).

use std::collections::BTreeMap;

use mushaf_corpus::{Corpus, Files};

const SOURCES: &str = "sources:
  - id: book-a
    title: Book A
    role: truth
    locator: p<n>
    pins: [pin-a]
  - id: text-x
    title: Text X
    role: text
    locator: <key>
    pins: [pin-x]
";

const WORDS: &str = "# synthetic word table
word_id\tkind\ttext\tsep_before\tsep_after\tsource
1:1:1\tword\tabc\t\tU+0020\ttext-x:1
1:1:2\tword\tde\t\tU+0020\ttext-x:2
1:1:3\tayah_end\t1\t\t\ttext-x:3
1:2:1\tword\tfgh\t\tU+0020\ttext-x:4
";

const USUL: &str = "word_id\tletter\tcategory\tcontext\texception\tsource\tstatus\tsupport\tnote
1:1:1\t2\tcat_a\tboth\t\tbook-a:p3\traw-derived\tsingle-source\t
1:2:1\t1\tcat_a\twaqf\texcluded\tbook-a:p4 ; text-x:4\trule-derived\tcross-source\tan exception
";

fn files(extra: &[(&str, &str)]) -> Files {
    let mut f: Files = BTreeMap::new();
    f.insert("corpus/sources.yaml".into(), SOURCES.into());
    f.insert("corpus/text/words.tsv".into(), WORDS.into());
    for (k, v) in extra {
        f.insert((*k).into(), (*v).into());
    }
    f
}

fn errors(f: &Files) -> Vec<String> {
    match Corpus::load(f) {
        Err(e) => e.iter().map(ToString::to_string).collect(),
        Ok(c) => c.check().errors.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn a_well_formed_corpus_has_no_errors() {
    let f = files(&[("corpus/usul/cat_a.tsv", USUL)]);
    let c = Corpus::load(&f).unwrap();
    let r = c.check();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(c.words.len(), 4);
    assert_eq!(c.usul.len(), 2);
}

#[test]
fn an_empty_corpus_is_valid() {
    let mut f: Files = BTreeMap::new();
    f.insert("corpus/sources.yaml".into(), SOURCES.into());
    let r = Corpus::load(&f).unwrap().check();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
}

#[test]
fn owner_statuses_are_refused_in_data_files() {
    let bad = USUL.replace("raw-derived", "cross-checked");
    let e = errors(&files(&[("corpus/usul/cat_a.tsv", &bad)]));
    assert!(
        e.iter()
            .any(|m| m.contains("cat_a.tsv:2") && m.contains("cross-checked")),
        "{e:?}"
    );
}

#[test]
fn unregistered_sources_and_empty_locators_are_errors() {
    let bad = USUL
        .replace("book-a:p3", "book-z:p3")
        .replace("text-x:4", "text-x:");
    let e = errors(&files(&[("corpus/usul/cat_a.tsv", &bad)]));
    assert!(e.iter().any(|m| m.contains("book-z")), "{e:?}");
    assert!(e.iter().any(|m| m.contains("locator")), "{e:?}");
}

#[test]
fn references_to_missing_words_are_errors() {
    let bad = USUL.replace("1:2:1\t1", "9:9:9\t1");
    let e = errors(&files(&[("corpus/usul/cat_a.tsv", &bad)]));
    assert!(e.iter().any(|m| m.contains("9:9:9")), "{e:?}");
}

#[test]
fn a_letter_past_the_word_end_is_an_error() {
    // "fgh" has three clusters in this synthetic text; letter 7 does not exist.
    let bad = USUL.replace("1:2:1\t1", "1:2:1\t7");
    let e = errors(&files(&[("corpus/usul/cat_a.tsv", &bad)]));
    assert!(e.iter().any(|m| m.contains("letter 7")), "{e:?}");
}

#[test]
fn headers_must_match_exactly() {
    let bad = USUL.replacen("word_id\tletter", "word\tletter", 1);
    let e = errors(&files(&[("corpus/usul/cat_a.tsv", &bad)]));
    assert!(e.iter().any(|m| m.contains("header")), "{e:?}");
}

#[test]
fn rows_must_be_sorted_and_unique() {
    let swapped = WORDS.replace(
        "1:1:1\tword\tabc\t\tU+0020\ttext-x:1\n1:1:2\tword\tde\t\tU+0020\ttext-x:2\n",
        "1:1:2\tword\tde\t\tU+0020\ttext-x:2\n1:1:1\tword\tabc\t\tU+0020\ttext-x:1\n",
    );
    let mut f = files(&[]);
    f.insert("corpus/text/words.tsv".into(), swapped);
    assert!(errors(&f).iter().any(|m| m.contains("order")));

    let dup = USUL.replace("1:2:1\t1", "1:1:1\t2");
    let e = errors(&files(&[("corpus/usul/cat_a.tsv", &dup)]));
    assert!(e.iter().any(|m| m.contains("duplicate")), "{e:?}");
}

#[test]
fn the_category_column_must_match_the_file_name() {
    let e = errors(&files(&[(
        "corpus/usul/cat_b.tsv",
        &USUL.replace("cat_a", "cat_b").replacen("cat_b", "cat_a", 1),
    )]));
    assert!(e.iter().any(|m| m.contains("category")), "{e:?}");
}

#[test]
fn a_cell_with_a_wrong_column_count_is_an_error() {
    let bad = USUL.replace("\tan exception", "");
    let e = errors(&files(&[("corpus/usul/cat_a.tsv", &bad)]));
    assert!(e.iter().any(|m| m.contains("columns")), "{e:?}");
}

#[test]
fn approvals_are_current_stale_or_unknown() {
    let usul = ("corpus/usul/cat_a.tsv", USUL);
    let sha = mushaf_corpus::sha256_hex(USUL.as_bytes());
    let ok = format!(
        "- table: corpus/usul/cat_a.tsv\n  sha256: \"{sha}\"\n  status: cross-checked\n  by: Owner\n  date: 2026-01-01\n  except: [\"1:2:1:1\"]\n"
    );
    let c = Corpus::load(&files(&[usul, ("data/approvals/corpus.yaml", &ok)])).unwrap();
    let r = c.check();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(
        r.approvals.get("corpus/usul/cat_a.tsv").map(String::as_str),
        Some("current")
    );

    let stale = ok.replace(&sha, &"0".repeat(64));
    let c = Corpus::load(&files(&[usul, ("data/approvals/corpus.yaml", &stale)])).unwrap();
    assert_eq!(
        c.check()
            .approvals
            .get("corpus/usul/cat_a.tsv")
            .map(String::as_str),
        Some("stale")
    );

    let unknown = ok.replace("corpus/usul/cat_a.tsv", "corpus/usul/nope.tsv");
    let c = Corpus::load(&files(&[usul, ("data/approvals/corpus.yaml", &unknown)])).unwrap();
    assert!(
        c.check()
            .errors
            .iter()
            .any(|m| m.to_string().contains("nope.tsv"))
    );
}

#[test]
fn adad_rows_reference_words() {
    let adad = "surah\tayah\tfirst_word\tlast_word\tsource\tstatus
1\t1\t1:1:1\t1:1:2\tbook-a:p1\traw-derived
1\t2\t1:2:1\t1:9:9\tbook-a:p1\traw-derived
";
    let e = errors(&files(&[("corpus/adad/sys_a.tsv", adad)]));
    assert!(e.iter().any(|m| m.contains("1:9:9")), "{e:?}");
}

#[test]
fn position_zero_is_only_for_hizb_items() {
    let with_hizb = WORDS.replace(
        "1:2:1\tword",
        "1:2:0\thizb\tx\t\tU+0020\ttext-x:5\n1:2:1\tword",
    );
    let mut f = files(&[]);
    f.insert("corpus/text/words.tsv".into(), with_hizb.clone());
    assert!(errors(&f).is_empty(), "{:?}", errors(&f));

    f.insert(
        "corpus/text/words.tsv".into(),
        with_hizb.replace("1:2:0\thizb", "1:2:0\tword"),
    );
    assert!(errors(&f).iter().any(|m| m.contains("position 0")));
}
