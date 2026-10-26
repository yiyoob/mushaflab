//! T6.1: rules resolve exactly like hand-written patches. The synthetic profile r3 has only
//! the rule file `rules/r3.yaml` (Latin letters, rule 1): `r3.mark` adds U+0301 to "c" in
//! 1:1:2, `r3.cross` sets 1:1:2's separator to a space, `r3.toggle` (option `test.toggle`,
//! default off for r3) adds U+0302 to "a" in 1:1:1.

mod common;

use std::collections::BTreeMap;

use common::{item_text, patch, payload, synthetic};
use mushaf_model::PatchFile;
use mushaf_resolve::{LintNote, Mode, ResolveError, Selection, resolve, resolve_profile};

fn sel(choices: &[(&str, &str)]) -> Selection {
    Selection {
        riwayah: "r3".into(),
        tariq: None,
        choices: choices
            .iter()
            .map(|(g, c)| (g.to_string(), c.to_string()))
            .collect(),
    }
}

#[test]
fn rules_apply_through_the_profile() {
    let ds = synthetic();
    let r = resolve_profile(&ds, &sel(&[]), None, Mode::Data).unwrap();
    assert_eq!(item_text(&r, "1:1:2").unwrap(), "c\u{301}d ");
    assert_eq!(item_text(&r, "1:1:1").unwrap(), "ab\u{301} ");
    let on = resolve_profile(&ds, &sel(&[("test.toggle", "on")]), None, Mode::Data).unwrap();
    assert_eq!(item_text(&on, "1:1:1").unwrap(), "a\u{302}b\u{301} ");
    assert_eq!(on.choices["test.toggle"], "on");
}

#[test]
fn a_rule_equals_the_same_hand_written_patches() {
    let ds = synthetic();
    let by_rule = resolve_profile(&ds, &sel(&[("test.toggle", "on")]), None, Mode::Data).unwrap();
    let hand = PatchFile {
        spine: "test/v1".into(),
        patches: vec![
            patch(
                "h1",
                "riwayah",
                "set_marks",
                "1:1:2:1",
                &payload("{add: [\"U+0301\"]}"),
            ),
            patch(
                "h2",
                "riwayah",
                "replace",
                "1:1:2",
                &payload("{sep_after: \"U+0020\"}"),
            ),
            patch(
                "h3",
                "riwayah",
                "set_marks",
                "1:1:1:1",
                &payload("{add: [\"U+0302\"]}"),
            ),
        ],
    };
    let by_hand = resolve(
        &ds.spines["test/v1"],
        &[&hand],
        &BTreeMap::new(),
        &sel(&[]),
        None,
        Mode::Data,
    )
    .unwrap();
    let text = |r: &mushaf_resolve::Resolved| r.items.iter().map(|i| i.text()).collect::<String>();
    assert_eq!(text(&by_rule), text(&by_hand));
    let cluster = by_rule.items[1].clusters[0].clone();
    assert_eq!(cluster.provenance, ["r3.mark@1:1:2:1.set_marks"]);
}

#[test]
fn a_rule_conflicts_with_a_patch_on_the_same_target() {
    let mut ds = synthetic();
    ds.patch_files.insert(
        "patches/raw/r3.yaml".into(),
        PatchFile {
            spine: "test/v1".into(),
            patches: vec![patch(
                "p",
                "riwayah",
                "set_marks",
                "1:1:2:1",
                &payload("{add: [\"U+0303\"]}"),
            )],
        },
    );
    ds.profiles.get_mut("r3").unwrap().patch_stack = vec!["patches/raw/r3.yaml".into()];
    let e = resolve_profile(&ds, &sel(&[]), None, Mode::Data).unwrap_err();
    assert!(matches!(e, ResolveError::Conflict { .. }), "{e}");
}

#[test]
fn a_dead_rule_fails_data_mode_and_is_linted_in_free_mode() {
    let mut ds = synthetic();
    let f = ds.rule_files.get_mut("rules/r3.yaml").unwrap();
    f.rules[0].pattern = mushaf_model::from_yaml("[{base: [\"U+007A\"], capture: \"c\"}]").unwrap();
    let e = resolve_profile(&ds, &sel(&[]), None, Mode::Data).unwrap_err();
    assert!(matches!(e, ResolveError::Rule(_)), "{e}");
    let r = resolve_profile(&ds, &sel(&[]), None, Mode::Free).unwrap();
    assert!(r.lint.contains(&LintNote::RuleNoMatch {
        rule: "r3.mark".into()
    }));
}

#[test]
fn an_option_rule_must_be_in_the_groups_affects() {
    let mut ds = synthetic();
    ds.option_groups.get_mut("test.toggle").unwrap().affects = vec!["r1.opt.on".into()];
    let e = resolve_profile(&ds, &sel(&[]), None, Mode::Data).unwrap_err();
    assert!(matches!(e, ResolveError::NotInAffects { .. }), "{e}");
}
