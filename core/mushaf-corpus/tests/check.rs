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

// M1.3: ayah counting and layouts.

const ADAD: &str = "surah\tayah\tfirst_word\tlast_word\tsource\tstatus
1\t1\t1:1:1\t1:1:2\ttext-x:3\traw-derived
1\t2\t1:2:1\t1:2:1\ttext-x:4\traw-derived
";

#[test]
fn a_well_formed_ayah_count_has_no_errors() {
    let e = errors(&files(&[("corpus/adad/sys_a.tsv", ADAD)]));
    assert!(e.is_empty(), "{e:?}");
}

#[test]
fn ayah_rows_must_be_in_order_and_contiguous() {
    let gap = ADAD.replace("1\t2\t1:2:1", "1\t3\t1:2:1");
    let e = errors(&files(&[("corpus/adad/sys_a.tsv", &gap)]));
    assert!(e.iter().any(|m| m.contains("contiguous")), "{e:?}");

    let swapped = "surah\tayah\tfirst_word\tlast_word\tsource\tstatus
1\t2\t1:2:1\t1:2:1\ttext-x:4\traw-derived
1\t1\t1:1:1\t1:1:2\ttext-x:3\traw-derived
";
    let e = errors(&files(&[("corpus/adad/sys_a.tsv", swapped)]));
    assert!(e.iter().any(|m| m.contains("order")), "{e:?}");
}

#[test]
fn ayah_ranges_run_forward_over_word_items() {
    let backwards = ADAD.replace("1:1:1\t1:1:2", "1:1:2\t1:1:1");
    let e = errors(&files(&[("corpus/adad/sys_a.tsv", &backwards)]));
    assert!(
        e.iter().any(|m| m.contains("ends before it starts")),
        "{e:?}"
    );

    let marker = ADAD.replace("1:1:1\t1:1:2", "1:1:1\t1:1:3");
    let e = errors(&files(&[("corpus/adad/sys_a.tsv", &marker)]));
    assert!(e.iter().any(|m| m.contains("not a word item")), "{e:?}");

    let overlap = ADAD.replace("1\t2\t1:2:1", "1\t2\t1:1:2");
    let e = errors(&files(&[("corpus/adad/sys_a.tsv", &overlap)]));
    assert!(e.iter().any(|m| m.contains("overlaps")), "{e:?}");
}

const LAYOUT_YAML: &str = "layout_id: lay_a
script_id: script_a
edition: Edition A
pages: 1
lines_per_page: 4
writing_style: style_a
font: font_a
sources:
  - source: text-x
    pins: [pin-x]
    status: raw-derived
";

/// Words of two synthetic surahs, so a surah header can be checked.
const WORDS2: &str = "word_id\tkind\ttext\tsep_before\tsep_after\tsource
1:1:1\tword\tabc\t\tU+0020\ttext-x:1
1:1:2\tword\tde\t\tU+0020\ttext-x:2
1:1:3\tayah_end\t1\t\t\ttext-x:3
2:1:0\thizb\tx\t\tU+0020\ttext-x:4
2:1:1\tword\tfgh\t\tU+0020\ttext-x:5
2:1:2\tayah_end\t1\t\t\ttext-x:6
";

const LINES: &str = "page\tline\ttype\tfirst_word\tlast_word\tcentered
1\t1\ttext\t1:1:1\t1:1:3\ttrue
1\t2\tsurah_name\t\t\ttrue
1\t3\tbasmalah\t\t\ttrue
1\t4\ttext\t2:1:1\t2:1:2\tfalse
";

fn layout_files(yaml: &str, lines: &str) -> Files {
    let mut f = files(&[
        ("corpus/layouts/lay_a/layout.yaml", yaml),
        ("corpus/layouts/lay_a/lines.tsv", lines),
    ]);
    f.insert("corpus/text/words.tsv".into(), WORDS2.into());
    f
}

#[test]
fn a_well_formed_layout_has_no_errors() {
    let f = layout_files(LAYOUT_YAML, LINES);
    let c = Corpus::load(&f).unwrap();
    let r = c.check();
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(c.layouts.len(), 1);
    assert_eq!(c.layout_lines.len(), 4);
}

#[test]
fn the_layout_source_role_is_accepted() {
    let mut f = layout_files(LAYOUT_YAML, LINES);
    let src = SOURCES.to_string()
        + "  - id: lay-src\n    title: Layout\n    role: layout\n    locator: <page>\n    pins: [pin-l]\n";
    f.insert("corpus/sources.yaml".into(), src);
    let e = errors(&f);
    assert!(e.is_empty(), "{e:?}");
}

#[test]
fn a_lines_table_needs_its_layout_file() {
    let mut f = layout_files(LAYOUT_YAML, LINES);
    f.remove("corpus/layouts/lay_a/layout.yaml");
    let e = errors(&f);
    assert!(e.iter().any(|m| m.contains("layout.yaml")), "{e:?}");
}

#[test]
fn layout_file_sources_and_id_are_checked() {
    let e = errors(&layout_files(
        &LAYOUT_YAML.replace("source: text-x", "source: nope"),
        LINES,
    ));
    assert!(e.iter().any(|m| m.contains("nope")), "{e:?}");
    let e = errors(&layout_files(&LAYOUT_YAML.replace("lay_a", "lay_b"), LINES));
    assert!(e.iter().any(|m| m.contains("lay_b")), "{e:?}");
}

#[test]
fn layout_lines_must_cover_every_word_once_in_order() {
    let gap = LINES.replace("1:1:1\t1:1:3", "1:1:1\t1:1:2");
    let e = errors(&layout_files(LAYOUT_YAML, &gap));
    assert!(e.iter().any(|m| m.contains("1:1:3")), "{e:?}");

    let overlap = LINES.replace("2:1:1\t2:1:2", "1:1:3\t2:1:2");
    let e = errors(&layout_files(LAYOUT_YAML, &overlap));
    assert!(e.iter().any(|m| m.contains("expected 2:1:1")), "{e:?}");
}

#[test]
fn layout_line_types_and_numbers_are_checked() {
    let no_words = LINES.replace("text\t2:1:1\t2:1:2", "text\t\t");
    let e = errors(&layout_files(LAYOUT_YAML, &no_words));
    assert!(e.iter().any(|m| m.contains("needs a word range")), "{e:?}");

    let words_on_header = LINES.replace("basmalah\t\t", "basmalah\t2:1:1\t2:1:1");
    let e = errors(&layout_files(LAYOUT_YAML, &words_on_header));
    assert!(e.iter().any(|m| m.contains("no word range")), "{e:?}");

    let past = LINES.replace("1\t4\ttext", "1\t5\ttext");
    let e = errors(&layout_files(LAYOUT_YAML, &past));
    assert!(e.iter().any(|m| m.contains("lines_per_page")), "{e:?}");

    let page2 = LINES.replace("1\t4\ttext", "2\t1\ttext");
    let e = errors(&layout_files(LAYOUT_YAML, &page2));
    assert!(e.iter().any(|m| m.contains("pages")), "{e:?}");
}

#[test]
fn a_surah_header_is_followed_on_its_page_by_that_surah() {
    // The header sits before a line that does not start a surah.
    let bad = "page\tline\ttype\tfirst_word\tlast_word\tcentered
1\t1\tsurah_name\t\t\ttrue
1\t2\ttext\t1:1:2\t1:1:3\ttrue
";
    let mut f = layout_files(LAYOUT_YAML, bad);
    let w = WORDS2
        .replace("2:1:0\thizb\tx\t\tU+0020\ttext-x:4\n", "")
        .replace(
            "2:1:1\tword\tfgh\t\tU+0020\ttext-x:5\n2:1:2\tayah_end\t1\t\t\ttext-x:6\n",
            "",
        )
        .replace("1:1:1\tword\tabc\t\tU+0020\ttext-x:1\n", "");
    f.insert("corpus/text/words.tsv".into(), w);
    let e = errors(&f);
    assert!(e.iter().any(|m| m.contains("surah_name")), "{e:?}");

    // The header is the last line of its page.
    let bad = LINES.replace(
        "1\t3\tbasmalah\t\t\ttrue\n1\t4\ttext\t2:1:1\t2:1:2\tfalse\n",
        "",
    );
    let e = errors(&layout_files(LAYOUT_YAML, &bad));
    assert!(e.iter().any(|m| m.contains("surah_name")), "{e:?}");
}
