//! Patches and patch files (PROJECT_PLAN §3.2; ADR D9, D12).
//!
//! The wire form (`PatchWire`) mirrors the YAML/JSON files field for field and rejects
//! unknown fields. It is checked into the typed `Patch`, whose `Op` carries only the payload
//! fields that op allows, so a malformed patch never reaches the resolver.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ClusterKind, Cp, Cps, Error, ItemKind, Ref};

/// Who a patch belongs to (§3.2 `owner.level`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerLevel {
    Qiraah,
    Riwayah,
    Tariq,
    User,
}

/// Priority layer, lowest first: `user > option > tariq > riwayah > qiraah` (§3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Qiraah,
    Riwayah,
    Tariq,
    Option,
    User,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub level: OwnerLevel,
    pub id: String,
}

/// Provenance statuses allowed in patch files (rule 4). `cross-checked` and `muqri-verified`
/// are owner-only and live in `data/approvals/`; a patch file carrying them is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    RawDerived,
    RuleDerived,
    /// Edits in the user overlay (free mode), and only there.
    UserEdit,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::RawDerived => "raw-derived",
            Status::RuleDerived => "rule-derived",
            Status::UserEdit => "user-edit",
        }
    }
}

impl Serialize for Status {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "raw-derived" => Ok(Status::RawDerived),
            "rule-derived" => Ok(Status::RuleDerived),
            "user-edit" => Ok(Status::UserEdit),
            "cross-checked" | "muqri-verified" => Err(serde::de::Error::custom(format!(
                "status {s:?} is owner-only and lives in data/approvals/, never in a patch file \
                 (CLAUDE.md rule 4)"
            ))),
            _ => Err(serde::de::Error::custom(format!("unknown status {s:?}"))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub kind: String,
    #[serde(rename = "ref")]
    pub reference: String,
}

/// Makes a patch part of the option layer, active when `option` resolves to `choice`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct When {
    pub option: String,
    pub choice: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCluster {
    pub kind: ClusterKind,
    pub cps: Cps,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewItem {
    pub kind: ItemKind,
    #[serde(default, skip_serializing_if = "Cps::is_empty")]
    pub sep_before: Cps,
    pub sep_after: Cps,
    pub clusters: Vec<NewCluster>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewNode {
    Item(NewItem),
    Cluster(NewCluster),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Before,
    After,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkAt {
    pub cp: Cp,
    /// Mark index the new mark takes: 0 = directly after the cluster's first codepoint.
    pub at: u32,
}

/// One `set_marks` addition: a bare codepoint is appended after the existing marks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MarkAdd {
    Append(Cp),
    At(MarkAt),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttrName {
    WidthEmDelta,
    Stretch,
    Annotation(String),
}

impl fmt::Display for AttrName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttrName::WidthEmDelta => f.write_str("width_em_delta"),
            AttrName::Stretch => f.write_str("stretch"),
            AttrName::Annotation(k) => write!(f, "annotations.{k}"),
        }
    }
}

impl std::str::FromStr for AttrName {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        match s {
            "width_em_delta" => Ok(AttrName::WidthEmDelta),
            "stretch" => Ok(AttrName::Stretch),
            _ => match s.strip_prefix("annotations.") {
                Some(k) if !k.is_empty() => Ok(AttrName::Annotation(k.into())),
                _ => Err(Error::new(format!("unknown attribute {s:?}"))),
            },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttrChange {
    pub width_em_delta: Option<i32>,
    pub stretch: Option<i32>,
    pub annotations: BTreeMap<String, String>,
    pub unset: Vec<AttrName>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Delete an item or a cluster.
    Delete,
    /// Replace a cluster's kind and codepoints.
    ReplaceCluster(NewCluster),
    /// Replace an item's separators (its clusters change through cluster ops).
    ReplaceItem {
        sep_before: Option<Cps>,
        sep_after: Option<Cps>,
    },
    /// Insert a node with id `<target>+ins.<seq>` before or after the target.
    Insert {
        side: Side,
        seq: u32,
        node: NewNode,
    },
    /// Remove (first occurrence) then add marks; marks are a cluster's codepoints after the first.
    SetMarks {
        remove: Vec<Cp>,
        add: Vec<MarkAdd>,
    },
    ClearMarks,
    SetAttr(AttrChange),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PatchWire", into = "PatchWire")]
pub struct Patch {
    pub id: String,
    pub owner: Owner,
    pub target: Ref,
    pub op: Op,
    pub source: Source,
    pub status: Status,
    pub rule_id: Option<String>,
    pub when: Option<When>,
    /// No second independent source supports this patch (§3.5).
    pub single_source: bool,
}

impl Patch {
    pub fn layer(&self) -> Layer {
        if self.when.is_some() {
            return Layer::Option;
        }
        match self.owner.level {
            OwnerLevel::Qiraah => Layer::Qiraah,
            OwnerLevel::Riwayah => Layer::Riwayah,
            OwnerLevel::Tariq => Layer::Tariq,
            OwnerLevel::User => Layer::User,
        }
    }

    /// The id this patch claims in its layer: the new id for an insert, else the target.
    /// At most one patch per key per layer (D9).
    pub fn key(&self) -> Ref {
        match &self.op {
            Op::Insert { seq, .. } => self.target.inserted(*seq),
            _ => self.target.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OpKind {
    Delete,
    Replace,
    InsertBefore,
    InsertAfter,
    SetMarks,
    ClearMarks,
    SetAttr,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PayloadWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    seq: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    item: Option<NewItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cluster: Option<NewCluster>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<ClusterKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cps: Option<Cps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sep_before: Option<Cps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sep_after: Option<Cps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remove: Option<Vec<Cp>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    add: Option<Vec<MarkAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width_em_delta: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stretch: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    annotations: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unset: Option<Vec<String>>,
}

impl PayloadWire {
    fn present(&self) -> BTreeSet<&'static str> {
        let fields = [
            ("seq", self.seq.is_some()),
            ("item", self.item.is_some()),
            ("cluster", self.cluster.is_some()),
            ("kind", self.kind.is_some()),
            ("cps", self.cps.is_some()),
            ("sep_before", self.sep_before.is_some()),
            ("sep_after", self.sep_after.is_some()),
            ("remove", self.remove.is_some()),
            ("add", self.add.is_some()),
            ("width_em_delta", self.width_em_delta.is_some()),
            ("stretch", self.stretch.is_some()),
            ("annotations", self.annotations.is_some()),
            ("unset", self.unset.is_some()),
        ];
        fields
            .into_iter()
            .filter(|(_, p)| *p)
            .map(|(n, _)| n)
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchWire {
    id: String,
    owner: Owner,
    op: OpKind,
    target: Ref,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    payload: Option<PayloadWire>,
    source: Source,
    status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<When>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    single_source: bool,
}

fn op_from_wire(kind: OpKind, target: &Ref, p: PayloadWire) -> Result<Op, String> {
    let present = p.present();
    let only = |allowed: &[&str], required: &[&str]| -> Result<(), String> {
        if let Some(f) = present.iter().find(|f| !allowed.contains(f)) {
            return Err(format!("payload field {f:?} is not allowed for this op"));
        }
        if let Some(f) = required.iter().find(|f| !present.contains(*f)) {
            return Err(format!("payload field {f:?} is required for this op"));
        }
        if !required.is_empty() || allowed.is_empty() || !present.is_empty() {
            return Ok(());
        }
        Err("payload is empty".into())
    };
    let is_item = matches!(target, Ref::Item(_));
    match kind {
        OpKind::Delete => only(&[], &[]).map(|()| Op::Delete),
        OpKind::ClearMarks if !is_item => only(&[], &[]).map(|()| Op::ClearMarks),
        OpKind::Replace if is_item => {
            only(&["sep_before", "sep_after"], &[])?;
            Ok(Op::ReplaceItem {
                sep_before: p.sep_before,
                sep_after: p.sep_after,
            })
        }
        OpKind::Replace => {
            only(&["kind", "cps"], &["kind", "cps"])?;
            let (kind, cps) = p.kind.zip(p.cps).ok_or("kind and cps are required")?;
            let c = NewCluster { kind, cps };
            if c.cps.is_empty() {
                return Err("replacement cluster is empty".into());
            }
            Ok(Op::ReplaceCluster(c))
        }
        OpKind::InsertBefore | OpKind::InsertAfter => {
            let side = if kind == OpKind::InsertBefore {
                Side::Before
            } else {
                Side::After
            };
            let node = if is_item {
                only(&["seq", "item"], &["seq", "item"])?;
                let item = p.item.ok_or("item is required")?;
                if item.clusters.is_empty() || item.clusters.iter().any(|c| c.cps.is_empty()) {
                    return Err("inserted item needs non-empty clusters".into());
                }
                NewNode::Item(item)
            } else {
                only(&["seq", "cluster"], &["seq", "cluster"])?;
                let c = p.cluster.ok_or("cluster is required")?;
                if c.cps.is_empty() {
                    return Err("inserted cluster is empty".into());
                }
                NewNode::Cluster(c)
            };
            let seq = p.seq.ok_or("seq is required")?;
            if seq == 0 {
                return Err("seq must be >= 1".into());
            }
            Ok(Op::Insert { side, seq, node })
        }
        OpKind::SetMarks if !is_item => {
            only(&["remove", "add"], &[])?;
            let (remove, add) = (p.remove.unwrap_or_default(), p.add.unwrap_or_default());
            if remove.is_empty() && add.is_empty() {
                return Err("set_marks changes nothing".into());
            }
            Ok(Op::SetMarks { remove, add })
        }
        OpKind::SetAttr => {
            only(&["width_em_delta", "stretch", "annotations", "unset"], &[])?;
            let unset = p
                .unset
                .unwrap_or_default()
                .iter()
                .map(|s| s.parse::<AttrName>().map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            let change = AttrChange {
                width_em_delta: p.width_em_delta,
                stretch: p.stretch,
                annotations: p.annotations.unwrap_or_default(),
                unset,
            };
            let set: BTreeSet<AttrName> = change
                .width_em_delta
                .map(|_| AttrName::WidthEmDelta)
                .into_iter()
                .chain(change.stretch.map(|_| AttrName::Stretch))
                .chain(
                    change
                        .annotations
                        .keys()
                        .map(|k| AttrName::Annotation(k.clone())),
                )
                .collect();
            if let Some(n) = change.unset.iter().find(|n| set.contains(n)) {
                return Err(format!("attribute {n} is both set and unset"));
            }
            if set.is_empty() && change.unset.is_empty() {
                return Err("set_attr changes nothing".into());
            }
            Ok(Op::SetAttr(change))
        }
        OpKind::ClearMarks | OpKind::SetMarks => {
            Err("this op targets a cluster, not an item".into())
        }
    }
}

impl TryFrom<PatchWire> for Patch {
    type Error = Error;
    fn try_from(w: PatchWire) -> Result<Self, Error> {
        let err = |m: String| Error::new(format!("patch {:?}: {m}", w.id));
        if w.id.is_empty() || w.id.chars().any(char::is_whitespace) {
            return Err(err("id must be non-empty without whitespace".into()));
        }
        if w.owner.id.is_empty() {
            return Err(err("owner.id is empty".into()));
        }
        let user = w.owner.level == OwnerLevel::User;
        if user != (w.status == Status::UserEdit) {
            return Err(err(
                "status user-edit is for, and only for, owner.level user".into(),
            ));
        }
        if user && w.when.is_some() {
            return Err(err("user edits cannot depend on an option (`when`)".into()));
        }
        let op =
            op_from_wire(w.op, &w.target, w.payload.clone().unwrap_or_default()).map_err(err)?;
        Ok(Patch {
            id: w.id,
            owner: w.owner,
            target: w.target,
            op,
            source: w.source,
            status: w.status,
            rule_id: w.rule_id,
            when: w.when,
            single_source: w.single_source,
        })
    }
}

impl From<Patch> for PatchWire {
    fn from(p: Patch) -> Self {
        let mut pw = PayloadWire::default();
        let op = match p.op {
            Op::Delete => OpKind::Delete,
            Op::ClearMarks => OpKind::ClearMarks,
            Op::ReplaceCluster(c) => {
                (pw.kind, pw.cps) = (Some(c.kind), Some(c.cps));
                OpKind::Replace
            }
            Op::ReplaceItem {
                sep_before,
                sep_after,
            } => {
                (pw.sep_before, pw.sep_after) = (sep_before, sep_after);
                OpKind::Replace
            }
            Op::Insert { side, seq, node } => {
                pw.seq = Some(seq);
                match node {
                    NewNode::Item(i) => pw.item = Some(i),
                    NewNode::Cluster(c) => pw.cluster = Some(c),
                }
                if side == Side::Before {
                    OpKind::InsertBefore
                } else {
                    OpKind::InsertAfter
                }
            }
            Op::SetMarks { remove, add } => {
                pw.remove = (!remove.is_empty()).then_some(remove);
                pw.add = (!add.is_empty()).then_some(add);
                OpKind::SetMarks
            }
            Op::SetAttr(c) => {
                pw.width_em_delta = c.width_em_delta;
                pw.stretch = c.stretch;
                pw.annotations = (!c.annotations.is_empty()).then_some(c.annotations);
                pw.unset =
                    (!c.unset.is_empty()).then(|| c.unset.iter().map(|n| n.to_string()).collect());
                OpKind::SetAttr
            }
        };
        let payload = (!pw.present().is_empty()).then_some(pw);
        PatchWire {
            id: p.id,
            owner: p.owner,
            op,
            target: p.target,
            payload,
            source: p.source,
            status: p.status,
            rule_id: p.rule_id,
            when: p.when,
            single_source: p.single_source,
        }
    }
}

/// A patch file: the spine its ids refer to, and its patches (ids unique in the file).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PatchFileWire", into = "PatchFileWire")]
pub struct PatchFile {
    pub spine: String,
    pub patches: Vec<Patch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchFileWire {
    spine: String,
    patches: Vec<Patch>,
}

impl TryFrom<PatchFileWire> for PatchFile {
    type Error = Error;
    fn try_from(w: PatchFileWire) -> Result<Self, Error> {
        let mut seen = BTreeSet::new();
        if let Some(p) = w.patches.iter().find(|p| !seen.insert(p.id.as_str())) {
            return Err(Error::new(format!(
                "patch id {:?} occurs twice in one file",
                p.id
            )));
        }
        Ok(PatchFile {
            spine: w.spine,
            patches: w.patches,
        })
    }
}

impl From<PatchFile> for PatchFileWire {
    fn from(f: PatchFile) -> Self {
        PatchFileWire {
            spine: f.spine,
            patches: f.patches,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_json, from_yaml, to_json, to_yaml};

    const HEAD: &str = "spine: t/v1\npatches:\n";

    fn one(body: &str) -> Result<Patch, Error> {
        let f: PatchFile = from_yaml(&format!("{HEAD}{body}"))?;
        Ok(f.patches.into_iter().next().unwrap())
    }

    fn base(op: &str, target: &str, payload: &str) -> String {
        format!(
            "  - id: \"p1\"\n    owner: {{level: riwayah, id: \"r\"}}\n    op: {op}\n    \
             target: \"{target}\"\n{payload}    source: {{kind: \"test\", ref: \"fixture\"}}\n    \
             status: raw-derived\n"
        )
    }

    #[test]
    fn every_op_round_trips_through_yaml_and_json() {
        let cases = [
            base("delete", "1:1:1", ""),
            base("delete", "1:1:1:2", ""),
            base(
                "replace",
                "1:1:1:1",
                "    payload: {kind: letter, cps: \"U+0062 U+0301\"}\n",
            ),
            base("replace", "1:1:1", "    payload: {sep_after: \"U+00A0\"}\n"),
            base(
                "insert_after",
                "1:1:1",
                "    payload: {seq: 2, item: {kind: word, sep_after: \"U+0020\", clusters: [{kind: letter, cps: \"U+0078\"}]}}\n",
            ),
            base(
                "insert_before",
                "1:1:1:1+ins.1",
                "    payload: {seq: 1, cluster: {kind: letter, cps: \"U+0079\"}}\n",
            ),
            base(
                "set_marks",
                "1:1:1:1",
                "    payload: {remove: [\"U+0301\"], add: [\"U+0302\", {cp: \"U+0303\", at: 0}]}\n",
            ),
            base("clear_marks", "1:1:1:1", ""),
            base(
                "set_attr",
                "1:1:1",
                "    payload: {width_em_delta: -250, annotations: {madd: \"4\"}, unset: [stretch]}\n",
            ),
        ];
        for body in cases {
            let p = one(&body).unwrap_or_else(|e| panic!("{body}: {e}"));
            let file = PatchFile {
                spine: "t/v1".into(),
                patches: vec![p.clone()],
            };
            let back: PatchFile = from_yaml(&to_yaml(&file).unwrap()).unwrap();
            assert_eq!(back, file);
            let json = to_json(&file).unwrap();
            assert_eq!(from_json::<PatchFile>(&json).unwrap(), file);
            assert_eq!(
                to_json(&from_json::<PatchFile>(&json).unwrap()).unwrap(),
                json
            );
        }
    }

    #[test]
    fn key_and_layer() {
        let p = one(&base(
            "insert_after",
            "1:1:1:1",
            "    payload: {seq: 3, cluster: {kind: letter, cps: \"U+0078\"}}\n",
        ))
        .unwrap();
        assert_eq!(p.key().to_string(), "1:1:1:1+ins.3");
        assert_eq!(p.layer(), Layer::Riwayah);
        let mut q = p.clone();
        q.when = Some(When {
            option: "o".into(),
            choice: "c".into(),
        });
        assert_eq!(q.layer(), Layer::Option);
        assert!(
            Layer::Qiraah < Layer::Riwayah
                && Layer::Tariq < Layer::Option
                && Layer::Option < Layer::User
        );
    }

    #[test]
    fn rejects_bad_payloads() {
        for body in [
            base("delete", "1:1:1", "    payload: {seq: 1}\n"),
            base("replace", "1:1:1:1", "    payload: {cps: \"U+0062\"}\n"),
            base(
                "replace",
                "1:1:1:1",
                "    payload: {kind: letter, cps: \"\"}\n",
            ),
            base(
                "replace",
                "1:1:1",
                "    payload: {kind: letter, cps: \"U+0062\"}\n",
            ),
            base("replace", "1:1:1", ""),
            base(
                "insert_after",
                "1:1:1",
                "    payload: {seq: 1, cluster: {kind: letter, cps: \"U+0078\"}}\n",
            ),
            base(
                "insert_after",
                "1:1:1:1",
                "    payload: {seq: 0, cluster: {kind: letter, cps: \"U+0078\"}}\n",
            ),
            base(
                "insert_after",
                "1:1:1:1",
                "    payload: {cluster: {kind: letter, cps: \"U+0078\"}}\n",
            ),
            base(
                "insert_after",
                "1:1:1",
                "    payload: {seq: 1, item: {kind: word, sep_after: \"\", clusters: []}}\n",
            ),
            base("set_marks", "1:1:1", "    payload: {add: [\"U+0302\"]}\n"),
            base("set_marks", "1:1:1:1", "    payload: {}\n"),
            base(
                "set_marks",
                "1:1:1:1",
                "    payload: {add: [{cp: \"U+0302\", at: 1, x: 1}]}\n",
            ),
            base("clear_marks", "1:1:1", ""),
            base("set_attr", "1:1:1", "    payload: {width_em_delta: 0.5}\n"),
            base(
                "set_attr",
                "1:1:1",
                "    payload: {stretch: 1, unset: [stretch]}\n",
            ),
            base("set_attr", "1:1:1", "    payload: {unset: [color]}\n"),
            base("set_attr", "1:1:1", "    payload: {paylod: 1}\n"),
            base("delete", "1:1:1", "    extra: 1\n"),
            base("delete", "1:1", ""),
        ] {
            assert!(one(&body).is_err(), "accepted:\n{body}");
        }
    }

    #[test]
    fn owner_only_statuses_are_rejected() {
        for status in ["cross-checked", "muqri-verified"] {
            let body = base("delete", "1:1:1", "").replace("raw-derived", status);
            let e = one(&body).unwrap_err().to_string();
            assert!(e.contains("rule 4"), "{e}");
        }
    }

    #[test]
    fn user_edit_status_is_tied_to_the_user_level() {
        let user = base("delete", "1:1:1", "").replace("riwayah", "user");
        assert!(one(&user).is_err());
        assert!(one(&user.replace("raw-derived", "user-edit")).is_ok());
        assert!(one(&base("delete", "1:1:1", "").replace("raw-derived", "user-edit")).is_err());
        let with_when =
            user.replace("raw-derived", "user-edit") + "    when: {option: \"o\", choice: \"c\"}\n";
        assert!(one(&with_when).is_err());
    }

    #[test]
    fn yaml_guards() {
        // duplicate patch ids, duplicate keys, unquoted numbers/booleans where strings are expected
        let p = base("delete", "1:1:1", "");
        assert!(from_yaml::<PatchFile>(&format!("{HEAD}{p}{p}")).is_err());
        assert!(one(&p.replace("op: delete", "op: delete\n    op: delete")).is_err());
        assert!(one(&p.replace("id: \"r\"", "id: 12")).is_err());
        // YAML 1.2 reads an unquoted `yes` into a string field as "yes"; YAML 1.1 (PyYAML) would
        // read a boolean. compile_db rejects that case on the Python side (D12).
        assert!(one(&(p.clone() + "    single_source: yes\n")).is_err());
        assert!(
            one(&(p + "    single_source: true\n"))
                .unwrap()
                .single_source
        );
    }
}
