//! T2.6: line mapping. Synthetic layout `test-layout` (tests/fixtures/synthetic):
//!   1:1 surah_name | 1:2 text 1:1:1..1:1:3 | 1:3 text 1:2:0..1:2:3
//!   2:1 surah_name | 2:2 basmalah          | 2:3 text 2:1:1..2:1:3
//! Real: the identity-resolved Hafs spine on the 1441H layout.

use std::path::{Path, PathBuf};

use mushaf_layout::{LayoutError, LineNote, Mapping, map_lines};
use mushaf_model::{DataSet, Layout, LineKind, Patch, PatchFile, from_yaml};
use mushaf_resolve::{Mode, Resolved, resolve_patches};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn synthetic() -> DataSet {
    mushaf_db::load_dir(&repo().join("tests/fixtures/synthetic")).unwrap()
}

fn patch(id: &str, op: &str, target: &str, payload: &str) -> Patch {
    let payload = if payload.is_empty() {
        String::new()
    } else {
        format!("    payload: {payload}\n")
    };
    let y = format!(
        "spine: \"test/v1\"\npatches:\n  - id: \"{id}\"\n    owner: {{level: \"riwayah\", id: \"r\"}}\n    \
         op: \"{op}\"\n    target: \"{target}\"\n{payload}    source: {{kind: \"test\", ref: \"unit\"}}\n    \
         status: \"raw-derived\"\n"
    );
    from_yaml::<PatchFile>(&y).unwrap().patches.remove(0)
}

fn word(seq: u32) -> String {
    format!(
        "{{seq: {seq}, item: {{kind: \"word\", sep_after: \"U+0020\", clusters: [{{kind: \"letter\", cps: \"U+0071\"}}]}}}}"
    )
}

fn map(ps: &[Patch], mode: Mode) -> (Resolved, Mapping) {
    let ds = synthetic();
    let spine = &ds.spines["test/v1"];
    let r = resolve_patches(spine, &ps.iter().collect::<Vec<_>>(), mode).unwrap();
    let m = map_lines(&ds.layouts["test-layout"], spine, &r).unwrap();
    (r, m)
}

/// Item ids per line, as "page:line" -> ids.
fn lines(r: &Resolved, m: &Mapping) -> Vec<(String, Vec<String>)> {
    m.lines
        .iter()
        .map(|l| {
            let ids = r.items[l.items.clone()]
                .iter()
                .map(|i| i.id.to_string())
                .collect();
            (format!("{}:{}", l.page, l.line), ids)
        })
        .collect()
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn identity_follows_the_layout_anchors() {
    let (r, m) = map(&[], Mode::Data);
    assert_eq!(
        lines(&r, &m),
        [
            ("1:1".into(), s(&[])),
            ("1:2".into(), s(&["1:1:1", "1:1:2", "1:1:3"])),
            ("1:3".into(), s(&["1:2:0", "1:2:1", "1:2:2", "1:2:3"])),
            ("2:1".into(), s(&[])),
            ("2:2".into(), s(&[])),
            ("2:3".into(), s(&["2:1:1", "2:1:2", "2:1:3"])),
        ]
    );
    assert!(m.report.is_empty());
    assert!(matches!(m.lines[4].kind, LineKind::Basmalah));
}

// Owner decision at the Phase 2 gate (2026-10-22): an insert-before joins the line of the word
// it precedes, an insert-after the line of the word it follows; nested inserts follow their
// own anchor, so every insert goes to the line of the spine item its id starts with.

#[test]
fn an_insert_before_a_line_start_joins_that_line() {
    let (r, m) = map(
        &[patch("b", "insert_before", "1:2:0", &word(1))],
        Mode::Data,
    );
    let l = lines(&r, &m);
    assert_eq!(l[1].1, s(&["1:1:1", "1:1:2", "1:1:3"]));
    assert_eq!(
        l[2].1,
        s(&["1:2:0+ins.1", "1:2:0", "1:2:1", "1:2:2", "1:2:3"])
    );
}

#[test]
fn an_insert_after_a_line_end_stays_on_that_line() {
    let (r, m) = map(&[patch("a", "insert_after", "1:1:3", &word(1))], Mode::Data);
    let l = lines(&r, &m);
    assert_eq!(l[1].1, s(&["1:1:1", "1:1:2", "1:1:3", "1:1:3+ins.1"]));
    assert_eq!(l[2].1, s(&["1:2:0", "1:2:1", "1:2:2", "1:2:3"]));
}

#[test]
fn an_insert_before_the_first_spine_item_joins_its_line() {
    let (r, m) = map(
        &[patch("c", "insert_before", "1:1:1", &word(1))],
        Mode::Data,
    );
    assert_eq!(
        lines(&r, &m)[1].1,
        s(&["1:1:1+ins.1", "1:1:1", "1:1:2", "1:1:3"])
    );
}

#[test]
fn nested_inserts_follow_their_own_anchor() {
    let (r, m) = map(
        &[
            patch("a", "insert_after", "1:1:3", &word(1)),
            patch("b", "insert_before", "1:2:0", &word(1)),
            patch("a1", "insert_after", "1:1:3+ins.1", &word(1)),
            patch("a2", "insert_before", "1:1:3+ins.1", &word(2)),
            patch("b1", "insert_before", "1:2:0+ins.1", &word(1)),
            patch("b2", "insert_after", "1:2:0+ins.1", &word(2)),
        ],
        Mode::Data,
    );
    let l = lines(&r, &m);
    assert_eq!(
        l[1].1,
        s(&[
            "1:1:1",
            "1:1:2",
            "1:1:3",
            "1:1:3+ins.1+ins.2",
            "1:1:3+ins.1",
            "1:1:3+ins.1+ins.1"
        ])
    );
    assert_eq!(
        l[2].1,
        s(&[
            "1:2:0+ins.1+ins.1",
            "1:2:0+ins.1",
            "1:2:0+ins.1+ins.2",
            "1:2:0",
            "1:2:1",
            "1:2:2",
            "1:2:3"
        ])
    );
}

#[test]
fn deleted_anchors_fall_through_to_surviving_items() {
    let (r, m) = map(
        &[
            patch("a", "delete", "1:2:0", ""),
            patch("b", "delete", "1:2:3", ""),
            patch("c", "insert_after", "1:2:3", &word(1)),
        ],
        Mode::Data,
    );
    assert_eq!(lines(&r, &m)[2].1, s(&["1:2:1", "1:2:2", "1:2:3+ins.1"]));
}

#[test]
fn inserts_next_to_deleted_line_edge_anchors_keep_their_anchor_line() {
    // the anchors are deleted in the same layer (a replacement), the inserts stay on their line
    let (r, m) = map(
        &[
            patch("d0", "delete", "1:2:0", ""),
            patch("i0", "insert_before", "1:2:0", &word(1)),
            patch("d1", "delete", "1:1:3", ""),
            patch("i1", "insert_after", "1:1:3", &word(1)),
        ],
        Mode::Data,
    );
    let l = lines(&r, &m);
    assert_eq!(l[1].1, s(&["1:1:1", "1:1:2", "1:1:3+ins.1"]));
    assert_eq!(l[2].1, s(&["1:2:0+ins.1", "1:2:1", "1:2:2", "1:2:3"]));
}

#[test]
fn a_line_whose_anchors_are_all_deleted_is_reported() {
    let ps: Vec<Patch> = ["2:1:1", "2:1:2", "2:1:3"]
        .iter()
        .map(|t| patch(t, "delete", t, ""))
        .collect();
    let (r, m) = map(&ps, Mode::Data);
    assert_eq!(lines(&r, &m)[5].1, s(&[]));
    assert_eq!(m.report, [LineNote::EmptyLine { page: 2, line: 3 }]);
}

#[test]
fn layouts_must_tile_the_spine() {
    let ds = synthetic();
    let spine = &ds.spines["test/v1"];
    let r = resolve_patches(spine, &[], Mode::Data).unwrap();
    let with = |f: fn(&mut Layout)| {
        let mut l = ds.layouts["test-layout"].clone();
        f(&mut l);
        map_lines(&l, spine, &r)
    };
    fn set_first(l: &mut Layout, i: usize, id: &str) {
        if let LineKind::Text { first, .. } = &mut l.lines[i].kind {
            *first = id.parse().unwrap();
        }
    }
    assert!(matches!(
        with(|l| set_first(l, 2, "1:2:1")),
        Err(LayoutError::NotContiguous { .. })
    ));
    assert!(matches!(
        with(|l| set_first(l, 2, "1:9:1")),
        Err(LayoutError::UnknownAnchor { .. })
    ));
    assert!(matches!(
        with(|l| {
            l.lines.pop();
        }),
        Err(LayoutError::NotCovered)
    ));
    assert!(matches!(
        with(|l| l.spine = "test/v2".into()),
        Err(LayoutError::SpineMismatch { .. })
    ));
}

fn arb_patches() -> impl Strategy<Value = Vec<Patch>> {
    let ids = [
        "1:1:1", "1:1:2", "1:1:3", "1:2:0", "1:2:1", "1:2:2", "1:2:3", "2:1:1", "2:1:2", "2:1:3",
    ];
    prop::collection::vec((prop::sample::select(ids.to_vec()), 0u8..4, 1u32..3), 0..12).prop_map(
        |specs| {
            let mut seen = std::collections::BTreeSet::new();
            specs
                .into_iter()
                .enumerate()
                .filter_map(|(n, (t, op, seq))| {
                    let p = match op {
                        0 => patch(&format!("p{n}"), "delete", t, ""),
                        1 => patch(&format!("p{n}"), "insert_before", t, &word(seq)),
                        2 => patch(&format!("p{n}"), "insert_after", t, &word(seq)),
                        _ => patch(&format!("p{n}"), "delete", &format!("{t}:1"), ""),
                    };
                    seen.insert(p.key()).then_some(p)
                })
                .collect()
        },
    )
}

proptest! {
    #![proptest_config(Config { cases: 300, failure_persistence: None, rng_seed: RngSeed::Fixed(0x6c696e65), ..Config::default() })]

    #[test]
    fn every_item_is_in_exactly_one_line_in_order(ps in arb_patches()) {
        let ds = synthetic();
        let spine = &ds.spines["test/v1"];
        let refs: Vec<&Patch> = ps.iter().collect();
        let Ok(r) = resolve_patches(spine, &refs, Mode::Free) else { return Ok(()) }; // overlaps
        let m = map_lines(&ds.layouts["test-layout"], spine, &r).unwrap();
        let mut next = 0;
        for l in &m.lines {
            prop_assert_eq!(l.items.start, next, "lines are contiguous and in reading order");
            next = l.items.end;
        }
        prop_assert_eq!(next, r.items.len(), "every resolved item is in a line");
    }
}

#[test]
fn hafs_identity_maps_every_line_to_its_anchors() {
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let spine = &ds.spines["hafs/v1"];
    let layout = &ds.layouts["uthmani-1441h"];
    let r = resolve_patches(spine, &[], Mode::Data).unwrap();
    let m = map_lines(layout, spine, &r).unwrap();
    assert_eq!(m.lines.len(), 9046);
    assert!(m.report.is_empty());
    for (ml, l) in m.lines.iter().zip(&layout.lines) {
        match &l.kind {
            LineKind::Text { first, last } => {
                let items = &r.items[ml.items.clone()];
                assert_eq!(
                    (&items[0].id, &items[items.len() - 1].id),
                    (first, last),
                    "{}:{}",
                    l.page,
                    l.line
                );
            }
            _ => assert!(ml.items.is_empty()),
        }
    }
    assert_eq!(m.lines.last().unwrap().items.end, r.items.len());
}
