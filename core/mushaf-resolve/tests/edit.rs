//! T7.1: the free-edit API over the synthetic spine (Latin letters only, rule 1).

mod common;

use common::{item_text, spine};
use mushaf_model::{
    ClusterKind, ClusterRef, Cp, Cps, ItemKind, NewCluster, NewItem, NewNode, Patch, Ref, Side,
    to_json,
};
use mushaf_resolve::{EditError, EditSession, Mode, Resolved, resolve_patches};

fn cur(s: &EditSession) -> Resolved {
    let ps: Vec<&Patch> = s.overlay().patches.iter().collect();
    resolve_patches(&spine(), &ps, Mode::Free).unwrap()
}

fn c(id: &str) -> ClusterRef {
    id.parse().unwrap()
}

fn r(id: &str) -> Ref {
    id.parse().unwrap()
}

fn letter(text: &str) -> NewCluster {
    NewCluster {
        kind: ClusterKind::Letter,
        cps: Cps::from_text(text),
    }
}

fn session() -> EditSession {
    EditSession::new(spine().id.clone(), "tester")
}

#[test]
fn every_operation_edits_the_text() {
    let mut s = session();
    assert_eq!(item_text(&cur(&s), "1:1:1").unwrap(), "ab\u{301} ");
    s.replace_cluster(&cur(&s), c("1:1:1:1"), letter("x"))
        .unwrap();
    assert_eq!(item_text(&cur(&s), "1:1:1").unwrap(), "xb\u{301} ");
    let new = s
        .insert(
            &cur(&s),
            r("1:1:1:2"),
            Side::After,
            NewNode::Cluster(letter("y")),
        )
        .unwrap();
    assert_eq!(new, r("1:1:1:2+ins.1"));
    assert_eq!(item_text(&cur(&s), "1:1:1").unwrap(), "xb\u{301}y ");
    s.delete(&cur(&s), r("1:1:2:2")).unwrap();
    assert_eq!(item_text(&cur(&s), "1:1:2").unwrap(), "c\u{a0}");
    let word = s
        .insert(
            &cur(&s),
            r("1:1:1"),
            Side::After,
            NewNode::Item(NewItem {
                kind: ItemKind::Word,
                sep_before: Cps::default(),
                sep_after: Cps::from_text(" "),
                clusters: vec![letter("z")],
            }),
        )
        .unwrap();
    assert_eq!(item_text(&cur(&s), &word.to_string()).unwrap(), "z ");
    // a cluster of a word the session inserted is edited inside its insert
    s.replace_cluster(&cur(&s), c("1:1:1+ins.1:1"), letter("w"))
        .unwrap();
    assert_eq!(item_text(&cur(&s), "1:1:1+ins.1").unwrap(), "w ");
    assert_eq!(s.overlay().patches.len(), 4);
    assert!(cur(&s).report.is_empty(), "{:?}", cur(&s).report);
}

#[test]
fn mark_edits_are_exact_codepoints() {
    let mut s = session();
    // 1:2:1:1 is U+0065 U+0301 U+0302 in the synthetic spine
    s.add_mark(&cur(&s), c("1:2:1:1"), Cp('\u{303}'), Some(0))
        .unwrap();
    assert_eq!(
        item_text(&cur(&s), "1:2:1").unwrap(),
        "e\u{303}\u{301}\u{302}f "
    );
    s.remove_mark(&cur(&s), c("1:2:1:1"), Cp('\u{301}'))
        .unwrap();
    s.add_mark(&cur(&s), c("1:2:1:1"), Cp('\u{301}'), None)
        .unwrap();
    assert_eq!(
        item_text(&cur(&s), "1:2:1").unwrap(),
        "e\u{303}\u{302}\u{301}f "
    );
    assert_eq!(s.overlay().patches.len(), 1, "one replace on the cluster");
    assert_eq!(
        s.remove_mark(&cur(&s), c("1:2:1:2"), Cp('\u{301}')),
        Err(EditError::NoSuchMark {
            cluster: c("1:2:1:2"),
            cp: Cp('\u{301}')
        })
    );
}

#[test]
fn a_cluster_can_be_replaced_and_widened() {
    let mut s = session();
    s.replace_cluster(&cur(&s), c("1:1:1:1"), letter("x"))
        .unwrap();
    s.set_width(&cur(&s), r("1:1:1:1"), Some(120)).unwrap();
    s.set_stretch(&cur(&s), c("1:1:1:1"), Some(40)).unwrap();
    let res = cur(&s);
    let cl = &res.items[0].clusters[0];
    assert_eq!(
        (cl.cps.text(), cl.attrs.width_em_delta, cl.attrs.stretch),
        ("x".into(), Some(120), Some(40))
    );
    assert_eq!(s.overlay().patches.len(), 2);
    s.set_width(&cur(&s), r("1:1:1:1"), None).unwrap();
    s.set_stretch(&cur(&s), c("1:1:1:1"), None).unwrap();
    assert_eq!(
        s.overlay().patches.len(),
        1,
        "an empty attribute patch is dropped"
    );
    assert!(cur(&s).items[0].clusters[0].attrs.is_empty());
}

#[test]
fn non_canonical_edits_are_never_refused() {
    let mut s = session();
    // delete an ayah-end marker and a whole word, give a letter a sign's kind
    s.delete(&cur(&s), r("1:1:3")).unwrap();
    s.delete(&cur(&s), r("1:2:2")).unwrap();
    s.replace_cluster(
        &cur(&s),
        c("1:1:1:1"),
        NewCluster {
            kind: ClusterKind::Sign,
            cps: Cps::from_text("*"),
        },
    )
    .unwrap();
    let res = cur(&s);
    assert!(item_text(&res, "1:1:3").is_none() && item_text(&res, "1:2:2").is_none());
    assert!(!res.canonical());
}

#[test]
fn only_targets_that_are_not_there_are_errors() {
    let mut s = session();
    s.delete(&cur(&s), r("1:1:2")).unwrap();
    for target in ["1:1:2", "1:1:2:1", "9:9:9", "1:1:1:7"] {
        assert_eq!(
            s.set_width(&cur(&s), r(target), Some(1)),
            Err(EditError::NotFound(r(target)))
        );
    }
    assert_eq!(
        s.overlay().patches.len(),
        1,
        "a failed edit changes nothing"
    );
    assert!(matches!(
        s.replace_cluster(
            &cur(&s),
            c("1:1:1:1"),
            NewCluster {
                kind: ClusterKind::Letter,
                cps: Cps::default()
            }
        ),
        Err(EditError::Invalid(_))
    ));
}

#[test]
fn deleting_own_inserts_removes_them() {
    let mut s = session();
    let w = s
        .insert(
            &cur(&s),
            r("1:1:1"),
            Side::Before,
            NewNode::Item(NewItem {
                kind: ItemKind::Word,
                sep_before: Cps::default(),
                sep_after: Cps::from_text(" "),
                clusters: vec![letter("q")],
            }),
        )
        .unwrap();
    let inner = s
        .insert(
            &cur(&s),
            r(&format!("{w}:1")),
            Side::After,
            NewNode::Cluster(letter("p")),
        )
        .unwrap();
    s.set_width(&cur(&s), inner.clone(), Some(10)).unwrap();
    assert_eq!(s.overlay().patches.len(), 3);
    s.delete(&cur(&s), w).unwrap();
    assert!(s.overlay().patches.is_empty());
    assert!(cur(&s).canonical());
}

#[test]
fn deleting_an_item_drops_the_users_edits_inside_it() {
    let mut s = session();
    s.replace_cluster(&cur(&s), c("1:1:1:1"), letter("x"))
        .unwrap();
    s.set_width(&cur(&s), r("1:1:1:2"), Some(5)).unwrap();
    s.delete(&cur(&s), r("1:1:1")).unwrap();
    assert_eq!(s.overlay().patches.len(), 1);
    assert!(cur(&s).report.is_empty());
}

#[test]
fn undo_and_redo_restore_byte_identical_output() {
    let mut s = session();
    let mut states = vec![to_json(&cur(&s)).unwrap()];
    s.replace_cluster(&cur(&s), c("1:1:1:1"), letter("x"))
        .unwrap();
    states.push(to_json(&cur(&s)).unwrap());
    s.add_mark(&cur(&s), c("1:1:1:2"), Cp('\u{302}'), None)
        .unwrap();
    states.push(to_json(&cur(&s)).unwrap());
    s.delete(&cur(&s), r("1:1:2")).unwrap();
    states.push(to_json(&cur(&s)).unwrap());
    s.set_width(&cur(&s), r("1:2:1"), Some(250)).unwrap();
    states.push(to_json(&cur(&s)).unwrap());
    for want in states.iter().rev().skip(1) {
        assert!(s.undo());
        assert_eq!(&to_json(&cur(&s)).unwrap(), want);
    }
    assert!(!s.undo() && s.overlay().patches.is_empty());
    for want in states.iter().skip(1) {
        assert!(s.redo());
        assert_eq!(&to_json(&cur(&s)).unwrap(), want);
    }
    assert!(!s.redo());
    // a new edit after an undo clears the redo history
    s.undo();
    s.delete(&cur(&s), r("1:2:2")).unwrap();
    assert!(!s.can_redo());
    // an edit that changes nothing adds no history
    let n = s.overlay().clone();
    s.set_width(&cur(&s), r("1:1:1:2"), None).unwrap();
    assert_eq!(s.overlay(), &n);
    s.undo();
    assert_eq!(&to_json(&cur(&s)).unwrap(), &states[3]);
}
