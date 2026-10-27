//! T7.3: the lint layer for free edits (D54). Advisory only: it never blocks an edit and never
//! decides the "edited" marker, which any user patch or report entry turns on (D16, rule 11).
//!
//! An edit is judged against the canonical resolution of the same riwāyah and option set (the
//! stack without the overlay). For each item that differs, the lint also names the other
//! option choices whose canonical text has exactly that item, so an edit that reproduces a
//! known wajh is told apart from one that matches nothing.

use std::collections::BTreeMap;

use mushaf_model::{EditOverlay, ItemRef, OptionGroup};
use serde::Serialize;

use crate::{RItem, Resolved};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditChange {
    /// In both, but its text, separators or attributes differ.
    Changed,
    /// Only in the edited text.
    Inserted,
    /// Only in the canonical text.
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct EditLint {
    pub item: ItemRef,
    pub change: EditChange,
    /// Labels of the alternatives (e.g. `group=choice`) whose canonical text has this item
    /// exactly as edited (or, for a deletion, lacks it). Empty: canonical nowhere checked.
    pub matches: Vec<String>,
}

/// An item's content, without provenance (which patch made it is not what is read).
fn same(a: &RItem, b: &RItem) -> bool {
    let cl = |i: &RItem| -> Vec<_> {
        i.clusters
            .iter()
            .map(|c| (c.id.clone(), c.kind, c.cps.clone(), c.attrs.clone()))
            .collect()
    };
    (a.kind, &a.sep_before, &a.sep_after, &a.attrs)
        == (b.kind, &b.sep_before, &b.sep_after, &b.attrs)
        && cl(a) == cl(b)
}

fn index(r: &Resolved) -> BTreeMap<&ItemRef, &RItem> {
    r.items.iter().map(|i| (&i.id, i)).collect()
}

/// Every item where `edited` differs from `base` (the canonical text for the same riwāyah and
/// options), sorted by id, with the `alternatives` that have it as edited.
pub fn lint_edits(
    base: &Resolved,
    edited: &Resolved,
    alternatives: &[(String, &Resolved)],
) -> Vec<EditLint> {
    let mut out = Vec::new();
    let (bi, ei) = (index(base), index(edited));
    let alts: Vec<(&String, BTreeMap<&ItemRef, &RItem>)> =
        alternatives.iter().map(|(l, r)| (l, index(r))).collect();
    let matches = |id: &ItemRef, want: Option<&RItem>| -> Vec<String> {
        alts.iter()
            .filter(|(_, alt)| match (alt.get(id), want) {
                (Some(a), Some(w)) => same(a, w),
                (None, None) => true,
                _ => false,
            })
            .map(|(label, _)| (*label).clone())
            .collect()
    };
    for e in &edited.items {
        let change = match bi.get(&e.id) {
            Some(b) if same(b, e) => continue,
            Some(_) => EditChange::Changed,
            None => EditChange::Inserted,
        };
        out.push(EditLint {
            item: e.id.clone(),
            change,
            matches: matches(&e.id, Some(e)),
        });
    }
    for b in base.items.iter().filter(|b| !ei.contains_key(&b.id)) {
        out.push(EditLint {
            item: b.id.clone(),
            change: EditChange::Deleted,
            matches: matches(&b.id, None),
        });
    }
    out.sort();
    out
}

/// Lints an overlay through `resolve` (choices, overlay -> resolution; the caller's own
/// pipeline, so any restyling it does applies to every side alike). The base is `choices`
/// without the overlay; the alternatives are every other choice of each option group the
/// base depends on, one group at a time, labelled `group=choice`.
pub fn lint_overlay<E>(
    resolve: impl Fn(&BTreeMap<String, String>, Option<&EditOverlay>) -> Result<Resolved, E>,
    choices: &BTreeMap<String, String>,
    groups: &BTreeMap<String, OptionGroup>,
    overlay: &EditOverlay,
) -> Result<Vec<EditLint>, E> {
    let base = resolve(choices, None)?;
    let edited = resolve(choices, Some(overlay))?;
    let mut alts = Vec::new();
    for (g, chosen) in &base.choices {
        let Some(group) = groups.get(g) else { continue };
        for c in group.choices.iter().filter(|c| *c != chosen) {
            let mut with = choices.clone();
            with.insert(g.clone(), c.clone());
            alts.push((format!("{g}={c}"), resolve(&with, None)?));
        }
    }
    let alts: Vec<(String, &Resolved)> = alts.iter().map(|(l, r)| (l.clone(), r)).collect();
    Ok(lint_edits(&base, &edited, &alts))
}
