//! T5.1: every riwāyah profile loads and resolves in data mode. The resolved text of each
//! riwāyah is checked against KFGQPC by `just roundtrip`; here the profile path must give the
//! same result as resolving its patch files directly, and Ḥafṣ (no patches) is the spine.

mod common;

use std::collections::BTreeMap;

use common::{as_spine_items, repo, synthetic};
use mushaf_resolve::{Mode, ResolveError, Selection, resolve, resolve_profile};

fn sel(riwayah: &str) -> Selection {
    Selection {
        riwayah: riwayah.into(),
        ..Selection::default()
    }
}

#[test]
fn all_eight_profiles_load_and_resolve_in_data_mode() {
    let ds = mushaf_db::load_dir(&repo().join("data")).unwrap();
    let ids: Vec<_> = ds.profiles.keys().cloned().collect();
    assert_eq!(
        ids,
        [
            "bazzi",
            "duri_abuamr",
            "hafs",
            "qalun",
            "qunbul",
            "shuba",
            "susi",
            "warsh"
        ]
    );
    for (id, p) in &ds.profiles {
        let r = resolve_profile(&ds, &sel(id), None, Mode::Data).unwrap();
        let spine = &ds.spines[&p.spine];
        let files: Vec<_> = p.patch_stack.iter().map(|f| &ds.patch_files[f]).collect();
        let direct = resolve(spine, &files, &BTreeMap::new(), &sel(id), None, Mode::Data).unwrap();
        assert!(r.items == direct.items, "{id}: profile resolution differs");
        if id == "hafs" {
            assert!(as_spine_items(&r) == spine.items, "hafs is not the spine");
        } else {
            assert_ne!(r.text(), direct_spine_text(spine), "{id} resolved to Hafs");
        }
        assert_eq!(p.surah_headers.len(), 114, "{id}");
    }
}

fn direct_spine_text(spine: &mushaf_model::Spine) -> String {
    let r = mushaf_resolve::resolve_patches(spine, &[], Mode::Data).unwrap();
    r.text()
}

#[test]
fn synthetic_profile_stacks_its_files() {
    let ds = synthetic();
    let r = resolve_profile(&ds, &sel("r1"), None, Mode::Free).unwrap();
    let files: Vec<_> = ["patches/curated/q1.yaml", "patches/raw/r1.yaml"]
        .iter()
        .map(|f| &ds.patch_files[*f])
        .collect();
    let direct = resolve(
        &ds.spines["test/v1"],
        &files,
        &ds.option_groups,
        &sel("r1"),
        None,
        Mode::Free,
    )
    .unwrap();
    assert!(r.items == direct.items);
}

#[test]
fn unknown_riwayah_is_an_error() {
    let ds = synthetic();
    assert!(matches!(
        resolve_profile(&ds, &sel("nope"), None, Mode::Free),
        Err(ResolveError::UnknownRiwayah { .. })
    ));
}
