//! Rule evaluator (T6.1, ADR D38): matches declarative rules on a spine and expands every
//! match into ordinary patches, so the resolver applies rules exactly as hand-written patches.
//!
//! Matching is on the spine's own text (not on the output of other rules or patches; known
//! gap). It is deterministic: rules in file order, matches in spine order, leftmost and
//! non-overlapping per rule. Codepoints are compared exactly (no normalisation).
//!
//! A pattern is a sequence of cluster matchers. Consecutive matchers take consecutive
//! clusters of one item; a boundary moves from the last cluster of an item to the first
//! cluster of a later word (see `mushaf_model::Boundary`).

use std::collections::BTreeSet;

use mushaf_model::{
    At, Boundary, ClusterMatch, ClusterRef, Error, Item, ItemKind, Patch, PatchFile, Ref, Rule,
    RuleFile, Spine, Step,
};
use serde_json::{Value, json};

/// Something a data build must not accept, reported per rule (never silently dropped).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// The rule matches nowhere in its scope: a dead rule.
    NoMatch { rule: String },
    /// An exception names a place the pattern never matches: a stale exception.
    UnusedException { rule: String, cluster: ClusterRef },
}

/// The patches of a rule file and the notes found while expanding it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expansion {
    pub file: PatchFile,
    pub notes: Vec<Note>,
}

/// One match: the cluster bound to each capture name.
pub type Captures = Vec<(String, ClusterRef)>;

/// Expands every rule of `file` on `spine`.
pub fn expand(file: &RuleFile, spine: &Spine) -> Result<Expansion, Error> {
    if file.spine != spine.id {
        return Err(Error::new(format!(
            "rule file is for spine {:?}, not {:?}",
            file.spine, spine.id
        )));
    }
    let mut patches = Vec::new();
    let mut notes = Vec::new();
    for rule in &file.rules {
        let (found, skipped) = matches(rule, spine);
        if found.is_empty() && skipped.is_empty() {
            notes.push(Note::NoMatch {
                rule: rule.id.clone(),
            });
        }
        for e in &rule.except {
            if !skipped.contains(&e.cluster) {
                notes.push(Note::UnusedException {
                    rule: rule.id.clone(),
                    cluster: e.cluster.clone(),
                });
            }
        }
        for caps in &found {
            for a in &rule.actions {
                patches.push(patch(rule, a, caps)?);
            }
        }
    }
    Ok(Expansion {
        file: PatchFile {
            spine: spine.id.clone(),
            patches,
        },
        notes,
    })
}

fn patch(rule: &Rule, a: &mushaf_model::Action, caps: &Captures) -> Result<Patch, Error> {
    let (name, item) = a.capture();
    let cluster = caps
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, c)| c)
        .ok_or_else(|| Error::new(format!("rule {}: no capture {name:?}", rule.id)))?;
    let target = if item {
        Ref::Item(cluster.item.clone())
    } else {
        Ref::Cluster(cluster.clone())
    };
    let seq = a
        .payload
        .as_ref()
        .and_then(|p| p.get("seq"))
        .and_then(Value::as_u64);
    let key = seq.map_or_else(
        || target.to_string(),
        |s| target.inserted(s as u32).to_string(),
    );
    let mut wire = json!({
        "id": format!("{}@{}.{}", rule.id, key, a.op),
        "owner": rule.owner,
        "op": a.op,
        "target": target.to_string(),
        "source": {"kind": "rule", "ref": rule.id},
        "status": "rule-derived",
        "rule_id": rule.id,
    });
    if let Some(p) = &a.payload {
        wire["payload"] = p.clone();
    }
    if let Some(w) = &rule.when {
        wire["when"] = json!(w);
    }
    if rule.single_source {
        wire["single_source"] = json!(true);
    }
    serde_json::from_value(wire)
        .map_err(|e| Error::new(format!("rule {}: action {}: {e}", rule.id, a.op)))
}

/// All matches of a rule in spine order, and the places skipped by its exceptions (each
/// identified by the cluster an exception names). One cursor walks the spine's clusters; after
/// a match it resumes at the cluster after the match's last one.
pub fn matches(rule: &Rule, spine: &Spine) -> (Vec<Captures>, BTreeSet<ClusterRef>) {
    let except: BTreeSet<&ClusterRef> = rule.except.iter().map(|e| &e.cluster).collect();
    let in_scope = |item: &Item| {
        rule.scope.is_empty()
            || rule
                .scope
                .iter()
                .any(|s| s.contains(item.id.surah, item.id.ayah))
    };
    let mut found = Vec::new();
    let mut skipped = BTreeSet::new();
    let (mut i, mut k) = (0, 0);
    while i < spine.items.len() {
        if k >= spine.items[i].clusters.len() {
            (i, k) = (i + 1, 0);
            continue;
        }
        if in_scope(&spine.items[i])
            && let Some((caps, end)) = match_at(rule, spine, i, k)
        {
            let hit: Vec<&ClusterRef> = caps
                .iter()
                .map(|(_, c)| c)
                .filter(|c| except.contains(c))
                .collect();
            if hit.is_empty() {
                found.push(caps);
            } else {
                skipped.extend(hit.into_iter().cloned());
            }
            (i, k) = (end.0, end.1 + 1);
            continue;
        }
        k += 1;
    }
    (found, skipped)
}

fn marks(c: &mushaf_model::Cluster) -> &[char] {
    &c.cps.0[1..]
}

fn cluster_ok(m: &ClusterMatch, item: &Item, k: usize) -> bool {
    let c = &item.clusters[k];
    let ms = marks(c);
    let has = |cp: &mushaf_model::Cp| ms.contains(&cp.0);
    let last = item.clusters.len() - 1;
    (m.base.is_empty() || m.base.iter().any(|b| b.0 == c.cps.0[0]))
        && m.kind.is_none_or(|kind| kind == c.kind)
        && m.marks_all.iter().all(has)
        && (m.marks_any.is_empty() || m.marks_any.iter().any(has))
        && !m.marks_none.iter().any(has)
        && (!m.bare || ms.is_empty())
        && match m.at {
            None => true,
            Some(At::First) => k == 0,
            Some(At::Last) => k == last,
            Some(At::Only) => last == 0,
        }
}

/// The next word item after `i` across a boundary, if the boundary allows it.
fn cross(spine: &Spine, i: usize, b: Boundary) -> Option<usize> {
    let mut ayah_end = false;
    for (j, it) in spine.items.iter().enumerate().skip(i + 1) {
        match it.kind {
            ItemKind::Word => {
                return match b {
                    Boundary::Word if ayah_end => None,
                    Boundary::AyahEnd if !ayah_end => None,
                    _ => Some(j),
                };
            }
            ItemKind::AyahEnd => ayah_end = true,
            ItemKind::Hizb => {}
        }
    }
    None
}

/// Matches the rule's pattern starting at cluster `k` of item `i`; returns the captures and
/// the (item, cluster) of the last matched cluster.
fn match_at(rule: &Rule, spine: &Spine, i: usize, k: usize) -> Option<(Captures, (usize, usize))> {
    let (mut it, mut c) = (i, k);
    let mut caps = Vec::new();
    let mut first = true;
    let mut pending: Option<Boundary> = None;
    for step in &rule.pattern {
        match step {
            Step::Boundary(b) => pending = Some(*b),
            Step::Cluster(m) => {
                if !first {
                    match pending.take() {
                        Some(b) => {
                            if c + 1 != spine.items[it].clusters.len() {
                                return None;
                            }
                            it = cross(spine, it, b)?;
                            c = 0;
                        }
                        None => {
                            c += 1;
                            if c >= spine.items[it].clusters.len() {
                                return None;
                            }
                        }
                    }
                }
                first = false;
                if !cluster_ok(m, &spine.items[it], c) {
                    return None;
                }
                if let Some(name) = &m.capture {
                    caps.push((name.clone(), spine.items[it].clusters[c].id.clone()));
                }
            }
        }
    }
    Some((caps, (it, c)))
}

#[cfg(test)]
mod tests;
