//! Evaluator tests on a synthetic spine (made-up letter sequences, not Quranic text).

use mushaf_model::{
    Cluster, ClusterKind, Cps, Item, ItemKind, ItemRef, Op, RuleFile, Spine, from_yaml,
};

use super::*;

const BA: char = '\u{0628}';
const TA: char = '\u{062A}';
const THA: char = '\u{062B}';
const FATHA: char = '\u{064E}';
const KASRA: char = '\u{0650}';

fn item(s: u32, a: u32, n: u32, kind: ItemKind, clusters: &[&[char]]) -> Item {
    let id = ItemRef::spine(s, a, n);
    Item {
        clusters: clusters
            .iter()
            .enumerate()
            .map(|(k, cps)| Cluster {
                id: id.cluster(k as u32 + 1),
                kind: if kind == ItemKind::Word {
                    ClusterKind::Letter
                } else {
                    ClusterKind::Sign
                },
                cps: Cps(cps.to_vec()),
            })
            .collect(),
        id,
        kind,
        sep_before: Cps::default(),
        sep_after: Cps::from_text(" "),
    }
}

/// 1:1: [BA+FATHA TA] [BA TA+KASRA] <end>  hizb  1:2: [TA BA+FATHA] [THA] <end>
fn spine() -> Spine {
    let w = ItemKind::Word;
    let items = vec![
        item(1, 1, 1, w, &[&[BA, FATHA], &[TA]]),
        item(1, 1, 2, w, &[&[BA], &[TA, KASRA]]),
        item(1, 1, 3, ItemKind::AyahEnd, &[&['\u{06DD}']]),
        item(1, 2, 0, ItemKind::Hizb, &[&['\u{06DE}']]),
        item(1, 2, 1, w, &[&[TA], &[BA, FATHA]]),
        item(1, 2, 2, w, &[&[THA]]),
        item(1, 2, 3, ItemKind::AyahEnd, &[&['\u{06DD}']]),
    ];
    Spine::new("t/v1".into(), "test".into(), 1, items).unwrap()
}

fn file(rules: &str) -> RuleFile {
    from_yaml(&format!("spine: \"t/v1\"\nrules:\n{rules}")).unwrap()
}

fn rule(pattern: &str, extra: &str) -> String {
    format!(
        "  - id: \"t.r\"\n    owner: {{level: \"riwayah\", id: \"x\"}}\n    pattern: {pattern}\n{extra}    actions: [{{op: \"set_marks\", target: \"a\", payload: {{add: [\"U+0651\"]}}}}]\n"
    )
}

fn hits(pattern: &str, extra: &str) -> Vec<String> {
    let f = file(&rule(pattern, extra));
    let (found, _) = matches(&f.rules[0], &spine());
    found.iter().map(|c| c[0].1.to_string()).collect()
}

#[test]
fn single_cluster_marks_and_position() {
    assert_eq!(
        hits("[{base: [\"U+0628\"], capture: \"a\"}]", ""),
        ["1:1:1:1", "1:1:2:1", "1:2:1:2"]
    );
    assert_eq!(
        hits(
            "[{base: [\"U+0628\"], marks_all: [\"U+064E\"], capture: \"a\"}]",
            ""
        ),
        ["1:1:1:1", "1:2:1:2"]
    );
    assert_eq!(
        hits("[{base: [\"U+0628\"], bare: true, capture: \"a\"}]", ""),
        ["1:1:2:1"]
    );
    assert_eq!(
        hits(
            "[{marks_none: [\"U+064E\"], at: \"last\", capture: \"a\"}]",
            ""
        ),
        [
            "1:1:1:2", "1:1:2:2", "1:1:3:1", "1:2:0:1", "1:2:2:1", "1:2:3:1"
        ]
    );
    assert_eq!(
        hits("[{kind: \"letter\", at: \"only\", capture: \"a\"}]", ""),
        ["1:2:2:1"]
    );
}

#[test]
fn consecutive_clusters_stay_in_one_word() {
    // TA then BA: inside 1:2:1 only (1:1:1 ends with TA, and 1:1:2 starts with BA, but the
    // pattern has no boundary).
    assert_eq!(
        hits(
            "[{base: [\"U+062A\"], capture: \"a\"}, {base: [\"U+0628\"]}]",
            ""
        ),
        ["1:2:1:1"]
    );
}

#[test]
fn boundaries() {
    let p = |b: &str| {
        format!(
            "[{{base: [\"U+062A\"], at: \"last\", capture: \"a\"}}, {{boundary: \"{b}\"}}, {{capture: \"b\", at: \"first\"}}]"
        )
    };
    // Within the ayah: 1:1:1 -> 1:1:2.
    assert_eq!(hits(&p("word"), ""), ["1:1:1:2"]);
    // Across the ayah end (and the hizb sign): 1:1:2 -> 1:2:1.
    assert_eq!(hits(&p("ayah_end"), ""), ["1:1:2:2"]);
    assert_eq!(hits(&p("any"), ""), ["1:1:1:2", "1:1:2:2"]);
    let f = file(&rule(&p("ayah_end"), ""));
    let (found, _) = matches(&f.rules[0], &spine());
    assert_eq!(found[0][1].1.to_string(), "1:2:1:1");
}

#[test]
fn matches_are_leftmost_and_do_not_overlap() {
    // BA or TA, then any cluster: 1:1:1 is one match (BA TA); 1:1:2 another (BA TA).
    assert_eq!(
        hits("[{base: [\"U+0628\", \"U+062A\"], capture: \"a\"}, {}]", ""),
        ["1:1:1:1", "1:1:2:1", "1:2:1:1"]
    );
}

#[test]
fn scope_limits_the_start() {
    assert_eq!(
        hits(
            "[{base: [\"U+0628\"], capture: \"a\"}]",
            "    scope: [{surah: 1, from: 2}]\n"
        ),
        ["1:2:1:2"]
    );
}

#[test]
fn exceptions_and_notes() {
    let f = file(&rule(
        "[{base: [\"U+0628\"], capture: \"a\"}]",
        "    except: [{cluster: \"1:1:2:1\", reason: \"r\"}, {cluster: \"1:2:2:1\", reason: \"r\"}]\n",
    ));
    let e = expand(&f, &spine()).unwrap();
    let targets: Vec<String> = e
        .file
        .patches
        .iter()
        .map(|p| p.target.to_string())
        .collect();
    assert_eq!(targets, ["1:1:1:1", "1:2:1:2"]);
    assert_eq!(
        e.notes,
        [Note::UnusedException {
            rule: "t.r".into(),
            cluster: "1:2:2:1".parse().unwrap()
        }]
    );
    let dead = file(&rule("[{base: [\"U+0636\"], capture: \"a\"}]", ""));
    assert_eq!(
        expand(&dead, &spine()).unwrap().notes,
        [Note::NoMatch { rule: "t.r".into() }]
    );
}

#[test]
fn expansion_gives_checked_patches() {
    let f = file(
        "  - id: \"t.r\"\n    owner: {level: \"riwayah\", id: \"x\"}\n    when: {option: \"o\", choice: \"on\"}\n    pattern: [{base: [\"U+062B\"], capture: \"a\"}]\n    actions:\n      - {op: \"set_marks\", target: \"a\", payload: {add: [\"U+06DC\"]}}\n      - {op: \"insert_after\", target: \"a.item\", payload: {seq: 1, item: {kind: \"word\", sep_after: \"U+0020\", clusters: [{kind: \"letter\", cps: \"U+0628\"}]}}}\n",
    );
    let e = expand(&f, &spine()).unwrap();
    assert!(e.notes.is_empty());
    let [p, q] = e.file.patches.as_slice() else {
        panic!("{:?}", e.file.patches)
    };
    assert_eq!(p.id, "t.r@1:2:2:1.set_marks");
    assert_eq!(p.rule_id.as_deref(), Some("t.r"));
    assert_eq!(p.source.kind, "rule");
    assert_eq!(p.when.as_ref().unwrap().option, "o");
    assert!(matches!(p.op, Op::SetMarks { .. }));
    assert_eq!(q.id, "t.r@1:2:2+ins.1.insert_after");
    assert_eq!(q.target.to_string(), "1:2:2");
    for x in [p, q] {
        x.validate().unwrap();
    }
}

#[test]
fn a_bad_action_payload_is_an_error() {
    let f = file(
        &rule("[{base: [\"U+062B\"], capture: \"a\"}]", "")
            .replace("add: [\"U+0651\"]", "cps: \"U+0651\""),
    );
    assert!(expand(&f, &spine()).is_err());
}

#[test]
fn wrong_spine_is_an_error() {
    let mut f = file(&rule("[{base: [\"U+062B\"], capture: \"a\"}]", ""));
    f.spine = "other/v1".into();
    assert!(expand(&f, &spine()).is_err());
}

#[test]
fn expansion_is_deterministic() {
    let f = file(&rule(
        "[{base: [\"U+0628\", \"U+062A\"], capture: \"a\"}]",
        "",
    ));
    let a = mushaf_model::to_json(&expand(&f, &spine()).unwrap().file).unwrap();
    let b = mushaf_model::to_json(&expand(&f, &spine()).unwrap().file).unwrap();
    assert_eq!(a, b);
}
