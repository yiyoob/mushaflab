//! T7.3: the free-edit lint on the synthetic fixture (Latin letters only, rule 1). Group
//! `test.toggle` adds U+0307 to 2:1:2:1 when `on`; r1's default is `off`.

mod common;

use std::collections::BTreeMap;

use common::synthetic;
use mushaf_model::{ClusterKind, Cp, Cps, DataSet, EditOverlay, NewCluster, PatchFile};
use mushaf_resolve::{
    EditChange, EditLint, EditSession, Mode, ResolveError, Resolved, Selection, lint_overlay,
    resolve,
};

fn run(
    ds: &DataSet,
    choices: &BTreeMap<String, String>,
    overlay: Option<&EditOverlay>,
) -> Result<Resolved, ResolveError> {
    let files: Vec<&PatchFile> = ds.patch_files.values().collect();
    let sel = Selection {
        riwayah: "r1".into(),
        tariq: None,
        choices: choices.clone(),
    };
    let mode = if overlay.is_some() {
        Mode::Free
    } else {
        Mode::Data
    };
    resolve(
        &ds.spines["test/v1"],
        &files,
        &ds.option_groups,
        &sel,
        overlay,
        mode,
    )
}

fn lint(ds: &DataSet, s: &EditSession) -> Vec<EditLint> {
    lint_overlay(
        |c, o| run(ds, c, o),
        &BTreeMap::new(),
        &ds.option_groups,
        s.overlay(),
    )
    .unwrap()
}

#[test]
fn an_edit_that_reproduces_another_wajh_is_named() {
    let ds = synthetic();
    let none = BTreeMap::new();
    let cur = |s: &EditSession| run(&ds, &none, Some(s.overlay())).unwrap();
    let mut s = EditSession::new("test/v1", "me");
    s.add_mark(&cur(&s), "2:1:2:1".parse().unwrap(), Cp('\u{307}'), None)
        .unwrap();
    let l = lint(&ds, &s);
    assert_eq!(l.len(), 1);
    assert_eq!(
        (l[0].item.to_string(), l[0].change, l[0].matches.clone()),
        (
            "2:1:2".into(),
            EditChange::Changed,
            vec!["test.toggle=on".to_string()]
        )
    );
    // a different mark is canonical under no choice checked
    s.undo();
    s.add_mark(&cur(&s), "2:1:2:1".parse().unwrap(), Cp('\u{308}'), None)
        .unwrap();
    assert_eq!(lint(&ds, &s)[0].matches, Vec::<String>::new());
}

#[test]
fn insertions_deletions_and_no_ops() {
    let ds = synthetic();
    let none = BTreeMap::new();
    let cur = |s: &EditSession| run(&ds, &none, Some(s.overlay())).unwrap();
    let mut s = EditSession::new("test/v1", "me");
    // a replace that writes what is already there: a user patch, but no difference
    let c = &cur(&s).items[0].clusters[0].clone();
    s.replace_cluster(
        &cur(&s),
        c.id.clone(),
        NewCluster {
            kind: c.kind,
            cps: c.cps.clone(),
        },
    )
    .unwrap();
    assert!(lint(&ds, &s).is_empty());
    // the marker rule is unchanged: any user patch marks the page (D16, D54)
    assert!(!cur(&s).canonical());
    let id = cur(&s).items[1].id.clone();
    s.delete(&cur(&s), mushaf_model::Ref::Item(id.clone()))
        .unwrap();
    let anchor = cur(&s).items[0].clusters[0].id.clone();
    s.insert(
        &cur(&s),
        mushaf_model::Ref::Cluster(anchor.clone()),
        mushaf_model::Side::After,
        mushaf_model::NewNode::Cluster(NewCluster {
            kind: ClusterKind::Letter,
            cps: Cps::from_text("z"),
        }),
    )
    .unwrap();
    let l = lint(&ds, &s);
    let got: Vec<(String, EditChange)> = l.iter().map(|e| (e.item.to_string(), e.change)).collect();
    assert_eq!(
        got,
        vec![
            (anchor.item.to_string(), EditChange::Changed),
            (id.to_string(), EditChange::Deleted)
        ]
    );
}
