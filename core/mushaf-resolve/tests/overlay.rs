//! T2.5: the user edit overlay is the highest layer, over the synthetic fixture stack (riwayah
//! r1 replaces 1:1:1:1 with "x\u{301}", deletes 1:2:0, inserts 1:1:2+ins.1).

mod common;

use common::{item_text, patch, payload, synthetic};
use mushaf_model::{EditOverlay, PatchFile, from_yaml, to_yaml};
use mushaf_resolve::{Diagnostic, Mode, ResolveError, Resolved, Selection, resolve};

fn overlay(patches: Vec<mushaf_model::Patch>) -> EditOverlay {
    let mut o = EditOverlay::new("test/v1");
    for p in patches {
        o.upsert(p).unwrap();
    }
    o
}

fn run(o: &EditOverlay, mode: Mode) -> Result<Resolved, ResolveError> {
    let ds = synthetic();
    let files: Vec<&PatchFile> = ds.patch_files.values().collect();
    let sel = Selection {
        riwayah: "r1".into(),
        ..Selection::default()
    };
    resolve(
        &ds.spines["test/v1"],
        &files,
        &ds.option_groups,
        &sel,
        Some(o),
        mode,
    )
}

#[test]
fn the_overlay_overrides_lower_layers() {
    let o = overlay(vec![
        patch(
            "u1",
            "user",
            "replace",
            "1:1:1:1",
            &payload("{kind: \"letter\", cps: \"U+0079\"}"),
        ),
        patch(
            "u2",
            "user",
            "set_marks",
            "1:1:2+ins.1:1",
            &payload("{add: [\"U+0301\"]}"),
        ),
    ]);
    let r = run(&o, Mode::Data).unwrap();
    assert_eq!(item_text(&r, "1:1:1").unwrap(), "yb ");
    assert_eq!(item_text(&r, "1:1:2+ins.1").unwrap(), "9\u{301} ");
    assert_eq!(
        r.items[0].clusters[0].provenance,
        ["r1.replace.cluster", "u1"]
    );
}

#[test]
fn an_overlay_round_trips_through_its_patch_file() {
    let o = overlay(vec![patch("u1", "user", "delete", "1:1:1:2", "")]);
    let text = to_yaml(&o).unwrap();
    let back: EditOverlay = from_yaml(&text).unwrap();
    assert_eq!(back, o);
    assert_eq!(
        run(&back, Mode::Data).unwrap(),
        run(&o, Mode::Data).unwrap()
    );
}

#[test]
fn an_overlay_edit_on_what_a_lower_layer_deleted_is_an_orphan() {
    let o = overlay(vec![patch(
        "u1",
        "user",
        "set_attr",
        "1:2:0",
        &payload("{stretch: 1}"),
    )]);
    let r = run(&o, Mode::Free).unwrap();
    assert_eq!(
        r.report,
        [Diagnostic::Orphan {
            patch: "u1".into(),
            target: "1:2:0".parse().unwrap(),
            applied: false
        }]
    );
    assert!(matches!(run(&o, Mode::Data), Err(ResolveError::Strict(_))));
}

#[test]
fn user_patches_belong_only_in_the_overlay() {
    let mut ds = synthetic();
    let f = ds.patch_files.get_mut("patches/raw/r1.yaml").unwrap();
    f.patches.push(patch("u9", "user", "delete", "1:1:1", ""));
    let files: Vec<&PatchFile> = ds.patch_files.values().collect();
    let sel = Selection {
        riwayah: "r1".into(),
        ..Selection::default()
    };
    let r = resolve(
        &ds.spines["test/v1"],
        &files,
        &ds.option_groups,
        &sel,
        None,
        Mode::Free,
    );
    assert_eq!(
        r,
        Err(ResolveError::UserPatchInDataFile { patch: "u9".into() })
    );
}

#[test]
fn an_overlay_for_another_spine_is_an_error() {
    let mut o = overlay(vec![]);
    o.spine = "test/v2".into();
    assert!(matches!(
        run(&o, Mode::Free),
        Err(ResolveError::SpineMismatch { .. })
    ));
}
