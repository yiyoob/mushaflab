//! T2.4: option resolution on the synthetic fixture. Group `test.toggle` (choices off/on;
//! default off for riwayah r1, on for r1 tariq t2) switches patch `r1.opt.on`, which adds
//! U+0307 to cluster 2:1:2:1 ("k"). Group `test.other` has no defaults and no patches.

mod common;

use std::collections::BTreeMap;

use common::{item_text, synthetic};
use mushaf_model::{DataSet, PatchFile};
use mushaf_resolve::{LintNote, Mode, ResolveError, Resolved, Selection, resolve};

fn sel(riwayah: &str, tariq: Option<&str>, choices: &[(&str, &str)]) -> Selection {
    Selection {
        riwayah: riwayah.into(),
        tariq: tariq.map(String::from),
        choices: choices
            .iter()
            .map(|(g, c)| (g.to_string(), c.to_string()))
            .collect(),
    }
}

fn run_with(ds: &DataSet, s: &Selection) -> Result<Resolved, ResolveError> {
    let files: Vec<&PatchFile> = ds.patch_files.values().collect();
    resolve(
        &ds.spines["test/v1"],
        &files,
        &ds.option_groups,
        s,
        Mode::Data,
    )
}

fn run(s: &Selection) -> Result<Resolved, ResolveError> {
    run_with(&synthetic(), s)
}

const OFF: &str = "k\u{a0}";
const ON: &str = "k\u{307}\u{a0}";

#[test]
fn choice_precedence_selection_then_tariq_then_riwayah() {
    let r = run(&sel("r1", None, &[])).unwrap();
    assert_eq!(item_text(&r, "2:1:2").unwrap(), OFF);
    assert_eq!(
        r.choices,
        BTreeMap::from([("test.toggle".to_string(), "off".to_string())])
    );
    let r = run(&sel("r1", Some("t2"), &[])).unwrap();
    assert_eq!(item_text(&r, "2:1:2").unwrap(), ON);
    let r = run(&sel("r1", Some("t2"), &[("test.toggle", "off")])).unwrap();
    assert_eq!(item_text(&r, "2:1:2").unwrap(), OFF);
    let r = run(&sel("r1", Some("unknown-tariq"), &[])).unwrap();
    assert_eq!(
        item_text(&r, "2:1:2").unwrap(),
        OFF,
        "falls back to the riwayah default"
    );
}

#[test]
fn toggling_changes_only_the_affected_cluster() {
    let off = run(&sel("r1", None, &[])).unwrap();
    let on = run(&sel("r1", None, &[("test.toggle", "on")])).unwrap();
    let differ: Vec<String> = off
        .items
        .iter()
        .zip(&on.items)
        .filter(|(a, b)| a != b)
        .map(|(a, _)| a.id.to_string())
        .collect();
    assert_eq!(differ, ["2:1:2"]);
    assert_eq!(
        on.items
            .iter()
            .find(|i| i.id.to_string() == "2:1:2")
            .unwrap()
            .clusters[0]
            .provenance,
        ["r1.opt.on"]
    );
}

#[test]
fn no_selection_and_no_default_is_an_error_not_a_guess() {
    match run(&sel("r2", None, &[])) {
        Err(ResolveError::NoChoice {
            group,
            riwayah,
            tariq: None,
        }) => {
            assert_eq!((group.as_str(), riwayah.as_str()), ("test.toggle", "r2"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn groups_the_stack_does_not_use_need_no_choice() {
    let r = run(&sel("r1", None, &[])).unwrap();
    assert!(!r.choices.contains_key("test.other"));
    let r = run(&sel("r1", None, &[("test.other", "b")])).unwrap();
    assert_eq!(r.choices["test.other"], "b");
}

#[test]
fn unknown_groups_and_choices_are_errors() {
    assert!(matches!(
        run(&sel("r1", None, &[("nope", "on")])),
        Err(ResolveError::UnknownOption { .. })
    ));
    assert!(matches!(
        run(&sel("r1", None, &[("test.toggle", "maybe")])),
        Err(ResolveError::UnknownChoice { .. })
    ));
}

#[test]
fn allowed_for_is_advisory_only() {
    let r = run(&sel("r2", None, &[("test.toggle", "on")])).unwrap();
    assert_eq!(
        item_text(&r, "2:1:2").unwrap(),
        ON,
        "applied although r2 is not in allowed_for"
    );
    assert_eq!(
        r.lint,
        [LintNote::OptionNotAllowed {
            group: "test.toggle".into(),
            riwayah: "r2".into()
        }]
    );
    assert!(run(&sel("r1", None, &[])).unwrap().lint.is_empty());
}

#[test]
fn when_must_name_a_known_group_choice_and_be_in_affects() {
    let edit = |f: fn(&mut DataSet)| {
        let mut ds = synthetic();
        f(&mut ds);
        run_with(&ds, &sel("r1", None, &[]))
    };
    assert!(matches!(
        edit(|ds| {
            let f = ds.patch_files.get_mut("patches/raw/r1.yaml").unwrap();
            let p = f.patches.iter_mut().find(|p| p.id == "r1.opt.on").unwrap();
            p.when.as_mut().unwrap().option = "missing".into();
        }),
        Err(ResolveError::UnknownOption { .. })
    ));
    assert!(matches!(
        edit(|ds| {
            let f = ds.patch_files.get_mut("patches/raw/r1.yaml").unwrap();
            let p = f.patches.iter_mut().find(|p| p.id == "r1.opt.on").unwrap();
            p.when.as_mut().unwrap().choice = "sometimes".into();
        }),
        Err(ResolveError::UnknownChoice { .. })
    ));
    assert!(matches!(
        edit(|ds| ds
            .option_groups
            .get_mut("test.toggle")
            .unwrap()
            .affects
            .clear()),
        Err(ResolveError::NotInAffects { .. })
    ));
}

#[test]
fn a_patch_file_for_another_spine_is_an_error() {
    let mut ds = synthetic();
    ds.patch_files.get_mut("patches/raw/r1.yaml").unwrap().spine = "test/v2".into();
    assert!(matches!(
        run_with(&ds, &sel("r1", None, &[])),
        Err(ResolveError::SpineMismatch { .. })
    ));
}
