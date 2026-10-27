//! T2.3: the patch engine (§3.2, D9, D12).
//!
//! The spine becomes a tree with tombstones: every item and cluster node keeps its place when
//! deleted, and holds the nodes inserted before and after it, sorted by integer `seq`. Layers
//! apply lowest first; inside a layer inserts go first (shallowest anchor first, so an insert
//! may anchor on one made in the same layer), then modifications. Because keys are unique per
//! layer and deletions never move anything, the result does not depend on the order in which
//! patches are given. The output is the in-order walk of the live nodes.

use std::collections::BTreeMap;

use mushaf_model::{
    Attrs, ClusterKind, ClusterRef, Cps, ItemKind, ItemRef, Layer, MarkAdd, NewNode, Op, Patch,
    Ref, Side, Spine,
};

use crate::{Diagnostic, Mode, Origin, RCluster, RItem, ResolveError, Resolved};

struct CNode {
    id: ClusterRef,
    kind: ClusterKind,
    cps: Vec<char>,
    attrs: Attrs,
    prov: Vec<String>,
    deleted: bool,
    before: Vec<(u32, usize)>,
    after: Vec<(u32, usize)>,
}

struct INode {
    id: ItemRef,
    kind: ItemKind,
    sep_before: Cps,
    sep_after: Cps,
    attrs: Attrs,
    prov: Vec<String>,
    deleted: bool,
    /// Position in the spine, for spine items.
    spine_pos: Option<usize>,
    /// Cluster arena: the item's own clusters first (`roots` of them), then inserted ones.
    clusters: Vec<CNode>,
    roots: usize,
    before: Vec<(u32, usize)>,
    after: Vec<(u32, usize)>,
}

fn cnode(id: ClusterRef, kind: ClusterKind, cps: Vec<char>, prov: Vec<String>) -> CNode {
    CNode {
        id,
        kind,
        cps,
        attrs: Attrs::default(),
        prov,
        deleted: false,
        before: Vec::new(),
        after: Vec::new(),
    }
}

struct Tree<'s> {
    spine: &'s Spine,
    /// Spine items at their spine positions, then inserted items.
    items: Vec<INode>,
    inserted_items: BTreeMap<ItemRef, usize>,
    inserted_clusters: BTreeMap<ClusterRef, (usize, usize)>,
}

fn add_child(list: &mut Vec<(u32, usize)>, seq: u32, idx: usize) {
    let at = list.partition_point(|(s, _)| *s < seq);
    list.insert(at, (seq, idx));
}

fn ins_depth(r: &Ref) -> usize {
    match r {
        Ref::Item(i) => i.ins.len(),
        Ref::Cluster(c) => c.item.ins.len() + c.ins.len(),
    }
}

impl<'s> Tree<'s> {
    fn new(spine: &'s Spine) -> Self {
        let items = spine
            .items
            .iter()
            .enumerate()
            .map(|(pos, it)| INode {
                id: it.id.clone(),
                kind: it.kind,
                sep_before: it.sep_before.clone(),
                sep_after: it.sep_after.clone(),
                attrs: Attrs::default(),
                prov: Vec::new(),
                deleted: false,
                spine_pos: Some(pos),
                clusters: it
                    .clusters
                    .iter()
                    .map(|c| cnode(c.id.clone(), c.kind, c.cps.0.clone(), Vec::new()))
                    .collect(),
                roots: it.clusters.len(),
                before: Vec::new(),
                after: Vec::new(),
            })
            .collect();
        Tree {
            spine,
            items,
            inserted_items: BTreeMap::new(),
            inserted_clusters: BTreeMap::new(),
        }
    }

    fn item(&self, r: &ItemRef) -> Option<usize> {
        if r.is_spine() {
            self.spine.position(r)
        } else {
            self.inserted_items.get(r).copied()
        }
    }

    fn cluster(&self, r: &ClusterRef) -> Option<(usize, usize)> {
        if !r.ins.is_empty() {
            return self.inserted_clusters.get(r).copied();
        }
        let i = self.item(&r.item)?;
        let k = (r.n as usize).checked_sub(1)?;
        (k < self.items[i].roots).then_some((i, k))
    }

    /// Whether the node exists, and if so whether it is alive (not deleted, nor inside a
    /// deleted item).
    fn alive(&self, r: &Ref) -> Option<bool> {
        match r {
            Ref::Item(i) => self.item(i).map(|i| !self.items[i].deleted),
            Ref::Cluster(c) => self
                .cluster(c)
                .map(|(i, k)| !self.items[i].deleted && !self.items[i].clusters[k].deleted),
        }
    }

    fn exists(&self, r: &Ref) -> bool {
        self.alive(r).is_some()
    }

    fn insert(
        &mut self,
        p: &Patch,
        side: Side,
        seq: u32,
        node: &NewNode,
        report: &mut Vec<Diagnostic>,
    ) {
        let new_id = p.target.inserted(seq);
        let Some(alive) = self.alive(&p.target) else {
            report.push(Diagnostic::Unresolved {
                patch: p.id.clone(),
                target: p.target.clone(),
            });
            return;
        };
        if self.exists(&new_id) {
            report.push(Diagnostic::IdCollision {
                patch: p.id.clone(),
                id: new_id,
            });
            return;
        }
        let prov = vec![p.id.clone()];
        let visible = match (&p.target, node) {
            (Ref::Item(anchor), NewNode::Item(new)) => {
                let a = self.item(anchor).unwrap_or_default();
                let id = anchor.inserted(seq);
                let idx = self.items.len();
                self.items.push(INode {
                    id: id.clone(),
                    kind: new.kind,
                    sep_before: new.sep_before.clone(),
                    sep_after: new.sep_after.clone(),
                    attrs: Attrs::default(),
                    prov: prov.clone(),
                    deleted: false,
                    spine_pos: None,
                    clusters: new
                        .clusters
                        .iter()
                        .enumerate()
                        .map(|(k, c)| {
                            cnode(
                                id.cluster(k as u32 + 1),
                                c.kind,
                                c.cps.0.clone(),
                                prov.clone(),
                            )
                        })
                        .collect(),
                    roots: new.clusters.len(),
                    before: Vec::new(),
                    after: Vec::new(),
                });
                self.inserted_items.insert(id, idx);
                let anchor = &mut self.items[a];
                add_child(
                    if side == Side::Before {
                        &mut anchor.before
                    } else {
                        &mut anchor.after
                    },
                    seq,
                    idx,
                );
                true
            }
            (Ref::Cluster(anchor), NewNode::Cluster(new)) => {
                let (i, k) = self.cluster(anchor).unwrap_or_default();
                let id = anchor.inserted(seq);
                let item = &mut self.items[i];
                let idx = item.clusters.len();
                item.clusters
                    .push(cnode(id.clone(), new.kind, new.cps.0.clone(), prov));
                let a = &mut item.clusters[k];
                add_child(
                    if side == Side::Before {
                        &mut a.before
                    } else {
                        &mut a.after
                    },
                    seq,
                    idx,
                );
                self.inserted_clusters.insert(id, (i, idx));
                !item.deleted
            }
            // Patch validation ties the payload level to the target level.
            _ => unreachable!("insert payload level does not match its target"),
        };
        if !alive {
            report.push(Diagnostic::Orphan {
                patch: p.id.clone(),
                target: p.target.clone(),
                applied: visible,
            });
        }
    }

    fn modify(&mut self, p: &Patch, report: &mut Vec<Diagnostic>) {
        match self.alive(&p.target) {
            None => {
                report.push(Diagnostic::Unresolved {
                    patch: p.id.clone(),
                    target: p.target.clone(),
                });
                return;
            }
            Some(false) => {
                report.push(Diagnostic::Orphan {
                    patch: p.id.clone(),
                    target: p.target.clone(),
                    applied: false,
                });
                return;
            }
            Some(true) => {}
        }
        let mut ineffective = |reason: String| {
            report.push(Diagnostic::Ineffective {
                patch: p.id.clone(),
                target: p.target.clone(),
                reason,
            });
        };
        match &p.target {
            Ref::Item(r) => {
                let i = self.item(r).unwrap_or_default();
                let node = &mut self.items[i];
                match &p.op {
                    Op::Delete => node.deleted = true,
                    Op::ReplaceItem {
                        sep_before,
                        sep_after,
                    } => {
                        if let Some(s) = sep_before {
                            node.sep_before = s.clone();
                        }
                        if let Some(s) = sep_after {
                            node.sep_after = s.clone();
                        }
                    }
                    Op::SetAttr(change) => apply_attrs(&mut node.attrs, change, &mut ineffective),
                    _ => unreachable!("cluster op on an item"),
                }
                node.prov.push(p.id.clone());
            }
            Ref::Cluster(r) => {
                let (i, k) = self.cluster(r).unwrap_or_default();
                let node = &mut self.items[i].clusters[k];
                match &p.op {
                    Op::Delete => node.deleted = true,
                    Op::ReplaceCluster(c) => {
                        node.kind = c.kind;
                        node.cps = c.cps.0.clone();
                    }
                    Op::SetMarks { remove, add } => {
                        for cp in remove {
                            match node.cps.iter().skip(1).position(|c| *c == cp.0) {
                                Some(m) => {
                                    node.cps.remove(m + 1);
                                }
                                None => ineffective(format!("mark {cp} is not present")),
                            }
                        }
                        for a in add {
                            match a {
                                MarkAdd::Append(cp) => node.cps.push(cp.0),
                                MarkAdd::At(m) => {
                                    let marks = node.cps.len() - 1;
                                    if m.at as usize <= marks {
                                        node.cps.insert(m.at as usize + 1, m.cp.0);
                                    } else {
                                        node.cps.push(m.cp.0);
                                        ineffective(format!(
                                            "mark index {} is past the {marks} marks; appended",
                                            m.at
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    Op::ClearMarks => {
                        if node.cps.len() == 1 {
                            ineffective("the cluster has no marks".into());
                        }
                        node.cps.truncate(1);
                    }
                    Op::SetAttr(change) => apply_attrs(&mut node.attrs, change, &mut ineffective),
                    _ => unreachable!("item op on a cluster"),
                }
                node.prov.push(p.id.clone());
            }
        }
    }

    fn flatten(&self, report: &mut Vec<Diagnostic>) -> Vec<RItem> {
        let mut out = Vec::new();
        for pos in 0..self.spine.items.len() {
            self.walk_item(pos, pos, &mut out, report);
        }
        out
    }

    /// `root` is the spine position of the item whose subtree this is: every insert chain
    /// starts at a spine item, and that item's line is the insert's line (owner decision at
    /// the Phase 2 gate).
    fn walk_item(&self, i: usize, root: usize, out: &mut Vec<RItem>, report: &mut Vec<Diagnostic>) {
        let node = &self.items[i];
        for &(_, c) in &node.before {
            self.walk_item(c, root, out, report);
        }
        let origin = match node.spine_pos {
            Some(pos) => Origin::Spine { pos },
            None => Origin::Inserted { anchor: root },
        };
        if !node.deleted {
            let mut clusters = Vec::new();
            for k in 0..node.roots {
                walk_cluster(node, k, &mut clusters);
            }
            if clusters.is_empty() {
                report.push(Diagnostic::EmptyItem {
                    item: node.id.clone(),
                });
            }
            out.push(RItem {
                id: node.id.clone(),
                kind: node.kind,
                sep_before: node.sep_before.clone(),
                sep_after: node.sep_after.clone(),
                attrs: node.attrs.clone(),
                clusters,
                origin,
                provenance: node.prov.clone(),
            });
        }
        for &(_, c) in &node.after {
            self.walk_item(c, root, out, report);
        }
    }
}

fn walk_cluster(item: &INode, k: usize, out: &mut Vec<RCluster>) {
    let node = &item.clusters[k];
    for &(_, c) in &node.before {
        walk_cluster(item, c, out);
    }
    if !node.deleted {
        out.push(RCluster {
            id: node.id.clone(),
            kind: node.kind,
            cps: Cps(node.cps.clone()),
            attrs: node.attrs.clone(),
            provenance: node.prov.clone(),
        });
    }
    for &(_, c) in &node.after {
        walk_cluster(item, c, out);
    }
}

fn apply_attrs(
    attrs: &mut Attrs,
    change: &mushaf_model::AttrChange,
    ineffective: &mut impl FnMut(String),
) {
    use mushaf_model::AttrName;
    for name in &change.unset {
        let was = match name {
            AttrName::WidthEmDelta => attrs.width_em_delta.take().is_some(),
            AttrName::Stretch => attrs.stretch.take().is_some(),
            AttrName::Annotation(k) => attrs.annotations.remove(k).is_some(),
        };
        if !was {
            ineffective(format!("attribute {name} is not set"));
        }
    }
    if change.width_em_delta.is_some() {
        attrs.width_em_delta = change.width_em_delta;
    }
    if change.stretch.is_some() {
        attrs.stretch = change.stretch;
    }
    for (k, v) in &change.annotations {
        attrs.annotations.insert(k.clone(), v.clone());
    }
}

/// Static checks that make the result independent of patch order: unique patch ids, one
/// patch per key per layer (in the user layer, one content and one attribute patch, D53), and
/// no patch inside an item its own layer deletes.
fn check(patches: &[&Patch]) -> Result<(), ResolveError> {
    let mut ids = BTreeMap::new();
    let mut by_key: BTreeMap<(Layer, Ref, bool), Vec<&str>> = BTreeMap::new();
    for p in patches {
        p.validate()
            .map_err(|e| ResolveError::InvalidPatch(e.to_string()))?;
        if ids.insert(p.id.as_str(), ()).is_some() {
            return Err(ResolveError::DuplicatePatchId(p.id.clone()));
        }
        let (key, attr) = p.slot();
        by_key
            .entry((p.layer(), key, attr))
            .or_default()
            .push(&p.id);
    }
    if let Some(((layer, key, _), ids)) = by_key.iter().find(|(_, v)| v.len() > 1) {
        let mut ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
        ids.sort();
        return Err(ResolveError::Conflict {
            layer: *layer,
            key: Box::new(key.clone()),
            patches: ids,
        });
    }
    let deleted: BTreeMap<(Layer, Ref), &str> = patches
        .iter()
        .filter(|p| p.op == Op::Delete && matches!(p.target, Ref::Item(_)))
        .map(|p| ((p.layer(), p.target.clone()), p.id.as_str()))
        .collect();
    let mut overlaps: Vec<(Layer, Ref, String, String)> = patches
        .iter()
        .filter_map(|p| {
            let item = Ref::Item(p.key().item().clone());
            let d = deleted.get(&(p.layer(), item.clone()))?;
            p.key()
                .is_inside(&item)
                .then(|| (p.layer(), p.key(), d.to_string(), p.id.clone()))
        })
        .collect();
    overlaps.sort();
    if let Some((layer, key, deleting, patch)) = overlaps.into_iter().next() {
        return Err(ResolveError::Overlap {
            layer,
            key: Box::new(key),
            deleting,
            patch,
        });
    }
    Ok(())
}

/// Applies `patches` (all active) to the spine.
pub fn resolve_patches(
    spine: &Spine,
    patches: &[&Patch],
    mode: Mode,
) -> Result<Resolved, ResolveError> {
    check(patches)?;
    let mut tree = Tree::new(spine);
    let mut report = Vec::new();
    let mut layers: BTreeMap<Layer, Vec<&Patch>> = BTreeMap::new();
    for p in patches {
        layers.entry(p.layer()).or_default().push(p);
    }
    for (_, mut ps) in layers {
        ps.sort_by_key(|p| (ins_depth(&p.target), p.key()));
        for p in &ps {
            if let Op::Insert { side, seq, node } = &p.op {
                tree.insert(p, *side, *seq, node, &mut report);
            }
        }
        // Attribute patches last: in the user layer a key may also hold a content patch (D53).
        let modifies = ps.iter().filter(|p| !matches!(p.op, Op::Insert { .. }));
        let (attrs, content): (Vec<_>, Vec<_>) =
            modifies.partition(|p| matches!(p.op, Op::SetAttr(_)));
        for p in content.into_iter().chain(attrs) {
            tree.modify(p, &mut report);
        }
    }
    let items = tree.flatten(&mut report);
    report.sort();
    if mode == Mode::Data && !report.is_empty() {
        return Err(ResolveError::Strict(report));
    }
    let mut user_patches: Vec<String> = patches
        .iter()
        .filter(|p| p.layer() == Layer::User)
        .map(|p| p.id.clone())
        .collect();
    user_patches.sort();
    Ok(Resolved {
        items,
        report,
        choices: BTreeMap::new(),
        lint: Vec::new(),
        user_patches,
    })
}
