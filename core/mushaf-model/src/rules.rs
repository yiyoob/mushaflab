//! Rule files and the rule register (T6.1, ADR D38).
//!
//! A rule is a declarative pattern over the clusters of a spine, possibly across words, with
//! actions that are patch templates: each match becomes ordinary patches (`mushaf-rules`), so
//! the resolver applies rules exactly as it applies hand-written patches. This module holds
//! only the data types and their checks; matching lives in `mushaf-rules` (rule 7).
//!
//! Every rule has an entry in the register (`rules/register.yaml`): description, source and
//! status. `DataSet` validation fails if the two disagree.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{ClusterKind, ClusterRef, Cp, Error, Owner, OwnerLevel, When};

/// Where in its word a matched cluster must be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum At {
    First,
    Last,
    Only,
}

/// How a pattern moves from the last cluster of one item to the first cluster of a later word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Boundary {
    /// The next word in the same ayah (hizb signs in between are skipped).
    Word,
    /// The first word after an ayah-end item.
    AyahEnd,
    /// The next word, whether or not an ayah ends in between.
    Any,
}

/// One cluster of a pattern. Empty lists do not constrain.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterMatch {
    /// Allowed base codepoints (the cluster's first codepoint).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base: Vec<Cp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClusterKind>,
    /// Every one of these marks is present.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks_all: Vec<Cp>,
    /// At least one of these marks is present.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks_any: Vec<Cp>,
    /// None of these marks is present.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks_none: Vec<Cp>,
    /// The cluster has no marks at all.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bare: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<At>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Cluster(ClusterMatch),
    Boundary(Boundary),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryWire {
    boundary: Boundary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StepWire {
    Boundary(BoundaryWire),
    Cluster(ClusterMatch),
}

impl Serialize for Step {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Step::Cluster(c) => StepWire::Cluster(c.clone()),
            Step::Boundary(b) => StepWire::Boundary(BoundaryWire { boundary: *b }),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Step {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match StepWire::deserialize(d)? {
            StepWire::Boundary(b) => Step::Boundary(b.boundary),
            StepWire::Cluster(c) => Step::Cluster(c),
        })
    }
}

/// Ayahs a rule applies to: one surah, optionally an inclusive ayah range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub surah: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<u32>,
}

impl Scope {
    pub fn contains(&self, surah: u32, ayah: u32) -> bool {
        surah == self.surah
            && self.from.is_none_or(|f| ayah >= f)
            && self.to.is_none_or(|t| ayah <= t)
    }
}

/// A place where the pattern matches but the rule does not apply, with the reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    pub cluster: ClusterRef,
    pub reason: String,
}

/// A patch template. `target` names a capture (`n`: its cluster) or its item (`n.item`);
/// `op` and `payload` are those of a patch file (§3.2) and are checked as such per match.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub op: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl Eq for Action {}

impl Action {
    /// The capture name and whether the action targets the capture's item.
    pub fn capture(&self) -> (&str, bool) {
        match self.target.strip_suffix(".item") {
            Some(c) => (c, true),
            None => (self.target.as_str(), false),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RuleWire", into = "RuleWire")]
pub struct Rule {
    pub id: String,
    pub owner: Owner,
    pub when: Option<When>,
    pub pattern: Vec<Step>,
    pub scope: Vec<Scope>,
    pub except: Vec<Exception>,
    pub actions: Vec<Action>,
    /// No second independent source supports the rule (§3.5); copied to its patches.
    pub single_source: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleWire {
    id: String,
    owner: Owner,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<When>,
    pattern: Vec<Step>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    scope: Vec<Scope>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    except: Vec<Exception>,
    actions: Vec<Action>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    single_source: bool,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}

impl TryFrom<RuleWire> for Rule {
    type Error = Error;
    fn try_from(w: RuleWire) -> Result<Self, Error> {
        let err = |m: String| Error::new(format!("rule {:?}: {m}", w.id));
        if !valid_id(&w.id) {
            return Err(err(
                "id must be lowercase ASCII, digits, '.', '_' or '-'".into()
            ));
        }
        if w.owner.level == OwnerLevel::User {
            return Err(err("a rule cannot belong to the user layer".into()));
        }
        let first_cluster = matches!(w.pattern.first(), Some(Step::Cluster(_)));
        let last_cluster = matches!(w.pattern.last(), Some(Step::Cluster(_)));
        if !first_cluster || !last_cluster {
            return Err(err("a pattern starts and ends with a cluster".into()));
        }
        if w.pattern
            .windows(2)
            .any(|p| matches!(p, [Step::Boundary(_), Step::Boundary(_)]))
        {
            return Err(err("two boundaries in a row".into()));
        }
        let mut captures = BTreeSet::new();
        for s in &w.pattern {
            if let Step::Cluster(c) = s {
                if c.bare && !(c.marks_all.is_empty() && c.marks_any.is_empty()) {
                    return Err(err("`bare` contradicts required marks".into()));
                }
                if let Some(name) = &c.capture
                    && (!valid_id(name) || !captures.insert(name.as_str()))
                {
                    return Err(err(format!("capture {name:?} is invalid or repeated")));
                }
            }
        }
        if w.actions.is_empty() {
            return Err(err("no actions".into()));
        }
        for a in &w.actions {
            check_annotations(a).map_err(err)?;
            if !captures.contains(a.capture().0) {
                return Err(err(format!(
                    "action target {:?} is not a capture",
                    a.target
                )));
            }
        }
        if w.scope
            .iter()
            .any(|s| s.from.zip(s.to).is_some_and(|(f, t)| f > t))
        {
            return Err(err("scope range is reversed".into()));
        }
        let mut seen = BTreeSet::new();
        if let Some(e) = w.except.iter().find(|e| !seen.insert(&e.cluster)) {
            return Err(err(format!("exception {} listed twice", e.cluster)));
        }
        if let Some(e) = w.except.iter().find(|e| e.reason.trim().is_empty()) {
            return Err(err(format!("exception {} has no reason", e.cluster)));
        }
        Ok(Rule {
            id: w.id,
            owner: w.owner,
            when: w.when,
            pattern: w.pattern,
            scope: w.scope,
            except: w.except,
            actions: w.actions,
            single_source: w.single_source,
        })
    }
}

impl From<Rule> for RuleWire {
    fn from(r: Rule) -> Self {
        RuleWire {
            id: r.id,
            owner: r.owner,
            when: r.when,
            pattern: r.pattern,
            scope: r.scope,
            except: r.except,
            actions: r.actions,
            single_source: r.single_source,
        }
    }
}

/// The annotation channel (T6.3, ADR D40): pronunciation features that change no glyph are
/// written to a cluster's `annotations` through `set_attr`, never to its codepoints. Keys are
/// `pron.<feature>` from this closed list of technical terms; values are short ASCII tokens.
pub const PRONUNCIATION_FEATURES: [&str; 17] = [
    "ibdal", "idgham", "ikhfa", "imala", "iqlab", "ishmam", "izhar", "madd", "naql", "rawm",
    "sakt", "silah", "taghliz", "tahqiq", "taqlil", "tarqiq", "tashil",
];

/// The `pron.` key prefix of the annotation channel.
pub const PRONUNCIATION_PREFIX: &str = "pron.";

/// Checks that a `set_attr` action writes only annotation-channel keys with token values.
fn check_annotations(a: &Action) -> Result<(), String> {
    let Some(ann) = a.payload.as_ref().and_then(|p| p.get("annotations")) else {
        return Ok(());
    };
    let map = ann.as_object().ok_or("annotations must be a mapping")?;
    for (k, v) in map {
        let feature = k
            .strip_prefix(PRONUNCIATION_PREFIX)
            .filter(|f| PRONUNCIATION_FEATURES.contains(f))
            .ok_or_else(|| format!("annotation key {k:?} is not pron.<feature> (D40)"))?;
        let token = v.as_str().filter(|t| {
            !t.is_empty()
                && t.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
        });
        if token.is_none() {
            return Err(format!(
                "annotation {feature}: value {v} is not a short ASCII token"
            ));
        }
    }
    Ok(())
}

/// `rules/<name>.yaml`: rules on one spine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleFile {
    pub spine: String,
    pub rules: Vec<Rule>,
}

/// Where a rule comes from: a book and page (rule 1). `None` in the register makes the rule
/// a hypothesis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub book: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<u32>,
    pub page: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleStatus {
    /// Cited; its patches are `rule-derived`.
    RuleDerived,
    /// Not cited (yet): kept for review, never counted as verified.
    Hypothesis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterEntry {
    pub id: String,
    pub owner: Owner,
    pub description: String,
    /// The option group the rule depends on (`when.option`), if any.
    pub option: Option<String>,
    pub source: Option<Citation>,
    pub status: RuleStatus,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub single_source: bool,
}

/// `rules/register.yaml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    pub rules: Vec<RegisterEntry>,
}

impl Register {
    /// Every disagreement between the rule files and the register (empty when they agree).
    pub fn check<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, &'a RuleFile)>,
    ) -> Vec<String> {
        let mut problems = Vec::new();
        let mut entries: BTreeMap<&str, &RegisterEntry> = BTreeMap::new();
        for e in &self.rules {
            if entries.insert(e.id.as_str(), e).is_some() {
                problems.push(format!("register: {} is listed twice", e.id));
            }
            if e.description.trim().is_empty() {
                problems.push(format!("register: {} has no description", e.id));
            }
            match (&e.source, e.status) {
                (None, RuleStatus::RuleDerived) => problems.push(format!(
                    "register: {} is rule-derived without a source (it is a hypothesis)",
                    e.id
                )),
                (Some(c), _) if c.book.trim().is_empty() || c.page.trim().is_empty() => {
                    problems.push(format!("register: {} has an incomplete source", e.id));
                }
                _ => {}
            }
        }
        let mut found = BTreeSet::new();
        for (path, f) in files {
            for r in &f.rules {
                if !found.insert(r.id.as_str()) {
                    problems.push(format!("{path}: rule {} is defined twice", r.id));
                    continue;
                }
                let Some(e) = entries.get(r.id.as_str()) else {
                    problems.push(format!("{path}: rule {} has no register entry", r.id));
                    continue;
                };
                if e.owner != r.owner {
                    problems.push(format!("rule {}: owner differs from the register", r.id));
                }
                if e.option.as_deref() != r.when.as_ref().map(|w| w.option.as_str()) {
                    problems.push(format!("rule {}: option differs from the register", r.id));
                }
                if e.single_source != r.single_source {
                    problems.push(format!(
                        "rule {}: single_source differs from the register",
                        r.id
                    ));
                }
            }
        }
        for id in entries.keys().filter(|id| !found.contains(*id)) {
            problems.push(format!("register: {id} has no rule"));
        }
        problems
    }

    pub fn entry(&self, id: &str) -> Option<&RegisterEntry> {
        self.rules.iter().find(|e| e.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::from_yaml;

    pub(crate) const FILE: &str = r#"spine: "t/v1"
rules:
  - id: "r.one"
    owner: {level: "riwayah", id: "r"}
    when: {option: "o", choice: "on"}
    pattern:
      - {base: ["U+0628"], marks_any: ["U+064E"], at: "last", capture: "a"}
      - {boundary: "word"}
      - {base: ["U+062A"], capture: "b"}
    scope: [{surah: 1, from: 1, to: 2}]
    except: [{cluster: "1:2:1:1", reason: "listed in the source"}]
    actions:
      - {op: "set_marks", target: "a", payload: {remove: ["U+064E"]}}
      - {op: "replace", target: "b.item", payload: {sep_after: ""}}
"#;

    pub(crate) const REGISTER: &str = r#"rules:
  - id: "r.one"
    owner: {level: "riwayah", id: "r"}
    description: "test rule"
    option: "o"
    source: {book: "test book", volume: 1, page: "7"}
    status: "rule-derived"
"#;

    fn file(text: &str) -> Result<RuleFile, Error> {
        from_yaml(text)
    }

    #[test]
    fn parses() {
        let f = file(FILE).unwrap();
        let r = &f.rules[0];
        assert_eq!(r.pattern.len(), 3);
        assert_eq!(r.pattern[1], Step::Boundary(Boundary::Word));
        assert_eq!(r.actions[1].capture(), ("b", true));
        assert!(r.scope[0].contains(1, 2) && !r.scope[0].contains(1, 3));
    }

    #[test]
    fn rejects_bad_rules() {
        for (from, to) in [
            ("id: \"r.one\"", "id: \"R One\""),
            ("level: \"riwayah\"", "level: \"user\""),
            ("target: \"a\"", "target: \"z\""),
            ("capture: \"b\"", "capture: \"a\""),
            ("from: 1, to: 2", "from: 3, to: 2"),
            ("reason: \"listed in the source\"", "reason: \" \""),
            (
                "marks_any: [\"U+064E\"]",
                "marks_any: [\"U+064E\"], bare: true",
            ),
            (
                "      - {boundary: \"word\"}\n",
                "      - {boundary: \"word\"}\n      - {boundary: \"any\"}\n",
            ),
            ("at: \"last\"", "at: \"middle\""),
            ("capture: \"b\"}", "capture: \"b\", extra: 1}"),
        ] {
            let bad = FILE.replace(from, to);
            assert_ne!(bad, FILE, "{from}");
            assert!(file(&bad).is_err(), "accepted {to}");
        }
        let edge = FILE.replace(
            "      - {base: [\"U+062A\"], capture: \"b\"}\n",
            "      - {base: [\"U+062A\"], capture: \"b\"}\n      - {boundary: \"word\"}\n",
        );
        assert!(file(&edge).is_err(), "pattern ending with a boundary");
    }

    #[test]
    fn annotation_actions_use_the_channel() {
        let ok = FILE.replace(
            "{op: \"set_marks\", target: \"a\", payload: {remove: [\"U+064E\"]}}",
            "{op: \"set_attr\", target: \"a\", payload: {annotations: {\"pron.taqlil\": \"on\"}}}",
        );
        assert_ne!(ok, FILE);
        file(&ok).unwrap();
        for (bad, why) in [
            ("\"pron.taqlil\": \"on\"", "\"colour\": \"on\""),
            ("\"pron.taqlil\": \"on\"", "\"pron.unknown\": \"on\""),
            ("\"pron.taqlil\": \"on\"", "\"pron.taqlil\": \"Two Words\""),
        ] {
            let text = ok.replace(bad, why);
            assert!(file(&text).is_err(), "accepted {why}");
        }
    }

    #[test]
    fn register_agrees() {
        let f = file(FILE).unwrap();
        let reg: Register = from_yaml(REGISTER).unwrap();
        assert!(reg.check([("rules/r.yaml", &f)]).is_empty());
    }

    #[test]
    fn register_disagreements_are_reported() {
        let f = file(FILE).unwrap();
        let cases = [
            (
                REGISTER.replace("id: \"r.one\"", "id: \"r.two\""),
                "no register entry",
            ),
            (
                REGISTER.replace("id: \"r\"}", "id: \"q\"}"),
                "owner differs",
            ),
            (
                REGISTER.replace("option: \"o\"", "option: null"),
                "option differs",
            ),
            (
                REGISTER.replace(
                    "source: {book: \"test book\", volume: 1, page: \"7\"}",
                    "source: null",
                ),
                "without a source",
            ),
            (
                REGISTER.replace("description: \"test rule\"", "description: \"\""),
                "no description",
            ),
            (
                REGISTER.replace(
                    "status: \"rule-derived\"",
                    "status: \"rule-derived\"\n    single_source: true",
                ),
                "single_source differs",
            ),
        ];
        for (text, want) in cases {
            let reg: Register = from_yaml(&text).unwrap();
            let problems = reg.check([("rules/r.yaml", &f)]);
            assert!(
                problems.iter().any(|p| p.contains(want)),
                "{want}: {problems:?}"
            );
        }
        let empty = Register::default();
        assert!(empty.check([("rules/r.yaml", &f)])[0].contains("no register entry"));
        let reg: Register = from_yaml(REGISTER).unwrap();
        assert!(reg.check([])[0].contains("has no rule"));
    }

    #[test]
    fn hypothesis_needs_no_source() {
        let f = file(FILE).unwrap();
        let text = REGISTER
            .replace(
                "source: {book: \"test book\", volume: 1, page: \"7\"}",
                "source: null",
            )
            .replace("rule-derived", "hypothesis");
        let reg: Register = from_yaml(&text).unwrap();
        assert!(reg.check([("rules/r.yaml", &f)]).is_empty());
    }

    #[test]
    fn owner_statuses_are_not_rule_statuses() {
        for s in ["cross-checked", "muqri-verified"] {
            let text = REGISTER.replace("rule-derived", s);
            assert!(from_yaml::<Register>(&text).is_err(), "{s}");
        }
    }
}
