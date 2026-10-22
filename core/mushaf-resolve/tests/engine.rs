//! T2.3: one test per op, per error and per report kind, on the synthetic spine (Latin letters;
//! no Quranic text, rule 1). Spine `test/v1` (tests/fixtures/synthetic):
//!   1:1:1 "a" "b\u{301}" + " " | 1:1:2 "c" "d" + NBSP | 1:1:3 "1"
//!   1:2:0 "*" + NBSP | 1:2:1 "e\u{301}\u{302}" "f" + " " | 1:2:2 "g" "h" + NBSP | 1:2:3 "2"
//!   " " + 2:1:1 "i" "j" + " " | 2:1:2 "k" + NBSP | 2:1:3 "1"

mod common;

use common::*;
use mushaf_model::{Layer, Patch, Ref};
use mushaf_resolve::{Diagnostic, Mode, Origin, ResolveError, resolve_patches};

fn run(ps: &[Patch], mode: Mode) -> Result<mushaf_resolve::Resolved, ResolveError> {
    resolve_patches(&spine(), &ps.iter().collect::<Vec<_>>(), mode)
}

fn ok(ps: &[Patch]) -> mushaf_resolve::Resolved {
    run(ps, Mode::Data).unwrap_or_else(|e| panic!("{e}"))
}

fn free(ps: &[Patch]) -> mushaf_resolve::Resolved {
    run(ps, Mode::Free).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn no_patches_gives_the_spine() {
    let r = ok(&[]);
    assert_eq!(as_spine_items(&r), spine().items);
    assert!(r.report.is_empty());
    let expected = "ab\u{301} cd\u{a0}1*\u{a0}e\u{301}\u{302}f gh\u{a0}2 ij k\u{a0}1";
    assert_eq!(r.text(), expected);
    assert!(
        r.items
            .iter()
            .enumerate()
            .all(|(n, i)| i.origin == Origin::Spine { pos: n })
    );
}

#[test]
fn delete_cluster_and_item() {
    let r = ok(&[
        patch("d1", "riwayah", "delete", "1:1:1:2", ""),
        patch("d2", "riwayah", "delete", "1:2:0", ""),
    ]);
    assert_eq!(item_text(&r, "1:1:1").unwrap(), "a ");
    assert_eq!(item_text(&r, "1:2:0"), None);
    assert_eq!(r.items.len(), 9);
}

#[test]
fn replace_cluster_and_item_separators() {
    let r = ok(&[
        patch(
            "r1",
            "riwayah",
            "replace",
            "1:1:1:1",
            &payload("{kind: \"letter\", cps: \"U+0078 U+0301\"}"),
        ),
        patch(
            "r2",
            "riwayah",
            "replace",
            "2:1:1",
            &payload("{sep_before: \"\", sep_after: \"U+00A0\"}"),
        ),
    ]);
    assert_eq!(item_text(&r, "1:1:1").unwrap(), "x\u{301}b\u{301} ");
    assert_eq!(item_text(&r, "2:1:1").unwrap(), "ij\u{a0}");
    let c = &r.items[0].clusters[0];
    assert_eq!(
        (c.id.to_string(), c.provenance.clone()),
        ("1:1:1:1".into(), vec!["r1".to_string()])
    );
}

fn word(seq: u32, letter: &str) -> String {
    payload(&format!(
        "{{seq: {seq}, item: {{kind: \"word\", sep_after: \"U+0020\", clusters: [{{kind: \"letter\", cps: \"{letter}\"}}]}}}}"
    ))
}

#[test]
fn inserted_items_are_ordered_by_seq_on_each_side() {
    let r = ok(&[
        patch("i2", "riwayah", "insert_after", "1:1:2", &word(2, "U+0071")),
        patch(
            "i3",
            "riwayah",
            "insert_before",
            "1:1:2",
            &word(3, "U+0070"),
        ),
        patch("i1", "riwayah", "insert_after", "1:1:2", &word(1, "U+0072")),
        patch(
            "i11",
            "riwayah",
            "insert_after",
            "1:1:2+ins.1",
            &word(1, "U+0073"),
        ),
    ]);
    assert_eq!(
        ids(&r)[..7],
        [
            "1:1:1",
            "1:1:2+ins.3",
            "1:1:2",
            "1:1:2+ins.1",
            "1:1:2+ins.1+ins.1",
            "1:1:2+ins.2",
            "1:1:3"
        ]
    );
    let ins = r
        .items
        .iter()
        .find(|i| i.id.to_string() == "1:1:2+ins.1+ins.1")
        .unwrap();
    assert_eq!(ins.clusters[0].id.to_string(), "1:1:2+ins.1+ins.1:1");
    assert_eq!(
        ins.origin,
        Origin::Inserted {
            after_spine: Some(1)
        }
    );
    let before = r
        .items
        .iter()
        .find(|i| i.id.to_string() == "1:1:2+ins.3")
        .unwrap();
    assert_eq!(
        before.origin,
        Origin::Inserted {
            after_spine: Some(0)
        }
    );
}

#[test]
fn insert_before_the_first_item_has_no_preceding_anchor() {
    let r = ok(&[patch(
        "i",
        "riwayah",
        "insert_before",
        "1:1:1",
        &word(1, "U+0071"),
    )]);
    assert_eq!(r.items[0].origin, Origin::Inserted { after_spine: None });
}

#[test]
fn inserted_clusters_and_nested_inserts() {
    let c = |seq: u32, cp: &str| {
        payload(&format!(
            "{{seq: {seq}, cluster: {{kind: \"letter\", cps: \"{cp}\"}}}}"
        ))
    };
    let r = ok(&[
        patch("a", "riwayah", "insert_after", "1:1:1:1", &c(1, "U+0078")),
        patch("b", "riwayah", "insert_before", "1:1:1:1", &c(2, "U+0077")),
        patch(
            "c",
            "riwayah",
            "insert_after",
            "1:1:1:1+ins.1",
            &c(1, "U+0079"),
        ),
        patch(
            "d",
            "tariq",
            "replace",
            "1:1:1:1+ins.1",
            &payload("{kind: \"letter\", cps: \"U+007A\"}"),
        ),
    ]);
    assert_eq!(item_text(&r, "1:1:1").unwrap(), "wazyb\u{301} ");
    let ids: Vec<String> = r.items[0]
        .clusters
        .iter()
        .map(|c| c.id.to_string())
        .collect();
    assert_eq!(
        ids,
        [
            "1:1:1:1+ins.2",
            "1:1:1:1",
            "1:1:1:1+ins.1",
            "1:1:1:1+ins.1+ins.1",
            "1:1:1:2"
        ]
    );
    assert_eq!(r.items[0].clusters[2].provenance, ["a", "d"]);
}

#[test]
fn set_marks_removes_first_occurrence_then_adds() {
    let r = ok(&[patch(
        "m",
        "riwayah",
        "set_marks",
        "1:2:1:1",
        &payload("{remove: [\"U+0301\"], add: [\"U+0303\", {cp: \"U+0304\", at: 0}]}"),
    )]);
    assert_eq!(item_text(&r, "1:2:1").unwrap(), "e\u{304}\u{302}\u{303}f ");
}

#[test]
fn set_marks_reports_what_it_could_not_do() {
    let p = patch(
        "m",
        "riwayah",
        "set_marks",
        "1:1:2:1",
        &payload("{remove: [\"U+0301\"], add: [{cp: \"U+0304\", at: 3}]}"),
    );
    let r = free(std::slice::from_ref(&p));
    assert_eq!(item_text(&r, "1:1:2").unwrap(), "c\u{304}d\u{a0}");
    assert_eq!(r.report.len(), 2);
    assert!(
        r.report
            .iter()
            .all(|d| matches!(d, Diagnostic::Ineffective { patch, .. } if patch == "m"))
    );
    assert!(matches!(
        run(&[p], Mode::Data),
        Err(ResolveError::Strict(_))
    ));
}

#[test]
fn clear_marks() {
    let r = ok(&[patch("c", "riwayah", "clear_marks", "1:2:1:1", "")]);
    assert_eq!(item_text(&r, "1:2:1").unwrap(), "ef ");
    let r = free(&[patch("c", "riwayah", "clear_marks", "1:1:1:1", "")]);
    assert!(matches!(&r.report[..], [Diagnostic::Ineffective { .. }]));
}

#[test]
fn set_attr_composes_across_layers() {
    let r = ok(&[
        patch(
            "q",
            "qiraah",
            "set_attr",
            "2:1:1",
            &payload("{stretch: 100, annotations: {a: \"1\", b: \"2\"}}"),
        ),
        patch(
            "r",
            "riwayah",
            "set_attr",
            "2:1:1",
            &payload("{width_em_delta: -50, unset: [\"annotations.a\"]}"),
        ),
        patch(
            "u",
            "user",
            "set_attr",
            "2:1:1:1",
            &payload("{width_em_delta: 30}"),
        ),
    ]);
    let it = r
        .items
        .iter()
        .find(|i| i.id.to_string() == "2:1:1")
        .unwrap();
    assert_eq!(
        (it.attrs.stretch, it.attrs.width_em_delta),
        (Some(100), Some(-50))
    );
    assert_eq!(it.attrs.annotations.keys().collect::<Vec<_>>(), ["b"]);
    assert_eq!(it.provenance, ["q", "r"]);
    assert_eq!(it.clusters[0].attrs.width_em_delta, Some(30));
    let r = free(&[patch(
        "r",
        "riwayah",
        "set_attr",
        "2:1:1",
        &payload("{unset: [\"stretch\"]}"),
    )]);
    assert!(matches!(&r.report[..], [Diagnostic::Ineffective { .. }]));
}

#[test]
fn higher_layers_apply_to_the_result_of_lower_ones() {
    let rep = |id: &str, level: &str, cp: &str| {
        patch(
            id,
            level,
            "replace",
            "1:1:2:2",
            &payload(&format!("{{kind: \"letter\", cps: \"{cp}\"}}")),
        )
    };
    let mut opt = rep("o", "riwayah", "U+0074");
    opt.when = Some(mushaf_model::When {
        option: "g".into(),
        choice: "c".into(),
    });
    let all = [
        rep("u", "user", "U+0075"),
        opt,
        rep("t", "tariq", "U+0073"),
        rep("r", "riwayah", "U+0072"),
        rep("q", "qiraah", "U+0071"),
    ];
    for n in 1..=all.len() {
        let r = ok(&all[all.len() - n..]);
        let want = ["u", "t", "s", "r", "q"][5 - n]; // the highest layer in the slice wins
        assert_eq!(item_text(&r, "1:1:2").unwrap(), format!("c{want}\u{a0}"));
    }
    let r = ok(&all);
    assert_eq!(r.items[1].clusters[1].provenance, ["q", "r", "t", "o", "u"]);
}

#[test]
fn same_key_in_one_layer_is_a_conflict() {
    let a = patch("a", "riwayah", "delete", "1:1:2:1", "");
    let b = patch(
        "b",
        "riwayah",
        "set_marks",
        "1:1:2:1",
        &payload("{add: [\"U+0301\"]}"),
    );
    match run(&[b.clone(), a.clone()], Mode::Free) {
        Err(ResolveError::Conflict {
            layer: Layer::Riwayah,
            key,
            patches,
        }) => {
            assert_eq!(
                (key.to_string(), patches),
                ("1:1:2:1".into(), vec!["a".into(), "b".into()])
            );
        }
        other => panic!("{other:?}"),
    }
    // the same key in different layers is fine
    let mut b2 = b;
    b2.owner.level = mushaf_model::OwnerLevel::Tariq;
    assert!(run(&[a, b2], Mode::Free).is_ok());
    // an insert's key is the id it creates: before and after with the same seq collide
    let i = patch("i", "riwayah", "insert_after", "1:1:2", &word(1, "U+0071"));
    let j = patch("j", "riwayah", "insert_before", "1:1:2", &word(1, "U+0072"));
    assert!(matches!(
        run(&[i, j], Mode::Free),
        Err(ResolveError::Conflict { .. })
    ));
}

#[test]
fn a_patch_inside_an_item_its_layer_deletes_is_an_error() {
    let ps = [
        patch("d", "riwayah", "delete", "1:1:1", ""),
        patch("m", "riwayah", "clear_marks", "1:1:1:2", ""),
    ];
    assert!(matches!(
        run(&ps, Mode::Free),
        Err(ResolveError::Overlap { .. })
    ));
}

#[test]
fn duplicate_patch_ids_and_invalid_patches_are_errors() {
    let ps = [
        patch("x", "riwayah", "delete", "1:1:1", ""),
        patch("x", "tariq", "delete", "1:1:2", ""),
    ];
    assert_eq!(
        run(&ps, Mode::Free),
        Err(ResolveError::DuplicatePatchId("x".into()))
    );
    let mut bad = patch("y", "riwayah", "delete", "1:1:1", "");
    bad.op = mushaf_model::Op::ClearMarks;
    assert!(matches!(
        run(&[bad], Mode::Free),
        Err(ResolveError::InvalidPatch(_))
    ));
}

#[test]
fn orphans_are_reported_never_dropped() {
    let del = patch("d", "riwayah", "delete", "1:1:1", "");
    let m = patch("m", "user", "clear_marks", "1:1:1:2", "");
    let r = free(&[del.clone(), m.clone()]);
    let target: Ref = "1:1:1:2".parse().unwrap();
    assert_eq!(
        r.report,
        [Diagnostic::Orphan {
            patch: "m".into(),
            target,
            applied: false
        }]
    );
    assert_eq!(item_text(&r, "1:1:1"), None);
    assert!(matches!(
        run(&[del, m], Mode::Data),
        Err(ResolveError::Strict(_))
    ));
}

#[test]
fn an_insert_next_to_a_deleted_anchor_is_placed_and_reported() {
    let r = free(&[
        patch("d", "riwayah", "delete", "1:1:2", ""),
        patch("i", "user", "insert_after", "1:1:2", &word(1, "U+0071")),
    ]);
    assert_eq!(ids(&r)[..3], ["1:1:1", "1:1:2+ins.1", "1:1:3"]);
    assert!(matches!(
        &r.report[..],
        [Diagnostic::Orphan { applied: true, .. }]
    ));
    // a cluster inserted into a deleted item is placed but not visible
    let c = payload("{seq: 1, cluster: {kind: \"letter\", cps: \"U+0078\"}}");
    let r = free(&[
        patch("d", "riwayah", "delete", "1:1:2", ""),
        patch("i", "user", "insert_after", "1:1:2:1", &c),
    ]);
    assert!(matches!(
        &r.report[..],
        [Diagnostic::Orphan { applied: false, .. }]
    ));
}

#[test]
fn delete_and_insert_at_one_anchor_in_one_layer_is_a_replacement() {
    let r = ok(&[
        patch("d", "riwayah", "delete", "1:1:2", ""),
        patch("i", "riwayah", "insert_after", "1:1:2", &word(1, "U+0071")),
    ]);
    assert_eq!(item_text(&r, "1:1:2+ins.1").unwrap(), "q ");
    assert_eq!(item_text(&r, "1:1:2"), None);
}

#[test]
fn unresolved_targets_and_id_collisions_are_reported() {
    let r = free(&[
        patch("u", "riwayah", "delete", "9:9:9", ""),
        patch("v", "riwayah", "delete", "1:1:1:3", ""),
        patch("w", "riwayah", "delete", "1:1:1+ins.4", ""),
        patch("i", "riwayah", "insert_after", "1:1:2", &word(1, "U+0071")),
        patch("j", "user", "insert_after", "1:1:2", &word(1, "U+0072")),
    ]);
    let kinds: Vec<(&str, String)> = r
        .report
        .iter()
        .map(|d| match d {
            Diagnostic::Unresolved { patch, target } => ("unresolved", format!("{patch} {target}")),
            Diagnostic::IdCollision { patch, id } => ("collision", format!("{patch} {id}")),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds.len(), 4);
    assert!(kinds.contains(&("collision", "j 1:1:2+ins.1".into())));
    assert!(kinds.contains(&("unresolved", "u 9:9:9".into())));
    assert_eq!(item_text(&r, "1:1:2+ins.1").unwrap(), "q ");
}

#[test]
fn an_item_left_without_clusters_is_reported() {
    let r = free(&[
        patch("a", "riwayah", "delete", "1:1:2:1", ""),
        patch("b", "riwayah", "delete", "1:1:2:2", ""),
    ]);
    assert_eq!(
        r.report,
        [Diagnostic::EmptyItem {
            item: "1:1:2".parse().unwrap()
        }]
    );
    assert_eq!(item_text(&r, "1:1:2").unwrap(), "\u{a0}");
}

#[test]
fn the_synthetic_fixture_stack_resolves_cleanly() {
    let ds = synthetic();
    let patches: Vec<&Patch> = ds.patch_files.values().flat_map(|f| &f.patches).collect();
    let r = resolve_patches(&ds.spines["test/v1"], &patches, Mode::Data)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(item_text(&r, "1:1:1").unwrap(), "x\u{301}b ");
    assert_eq!(item_text(&r, "1:1:2").unwrap(), "d ");
    assert_eq!(item_text(&r, "1:1:2+ins.1").unwrap(), "9 ");
    assert_eq!(item_text(&r, "1:2:0"), None);
    assert_eq!(item_text(&r, "1:2:1").unwrap(), "e\u{304}\u{302}\u{303}yf ");
    assert_eq!(item_text(&r, "2:1:2").unwrap(), "k\u{307}\u{a0}");
    let it = r
        .items
        .iter()
        .find(|i| i.id.to_string() == "2:1:1")
        .unwrap();
    assert_eq!(it.attrs.annotations.keys().collect::<Vec<_>>(), ["note"]);
    assert_eq!(
        (it.attrs.stretch, it.attrs.width_em_delta),
        (Some(500), Some(-120))
    );
}
