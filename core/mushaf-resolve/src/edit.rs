//! T7.1: the free-edit API (D53). An `EditSession` turns editing operations into user-layer
//! patches in an `EditOverlay`, with undo and redo.
//!
//! Rule 11: no operation is refused because its result is non-canonical. The only errors are
//! requests that name nothing: a target that is not in the current resolution (never existed,
//! or deleted), or a mark the cluster does not carry.
//!
//! Every operation takes the current resolution (`Resolved` of the stack with this session's
//! overlay), so the patch it writes says exactly what the user saw change. Undo and redo
//! restore whole overlay snapshots, so the same stack gives byte-identical output again
//! (rule 8).

use std::fmt;

use mushaf_model::{
    AttrChange, ClusterKind, ClusterRef, Cp, Cps, EditOverlay, ItemRef, NewCluster, NewNode, Op,
    Owner, OwnerLevel, Patch, Ref, Side, Source, Status,
};

use crate::{RCluster, RItem, Resolved};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// The target is not in the current resolution (it never existed, or it is deleted).
    NotFound(Ref),
    /// The cluster does not carry this mark.
    NoSuchMark { cluster: ClusterRef, cp: Cp },
    /// The patch built for the edit is malformed (for example an empty cluster).
    Invalid(String),
    /// An imported overlay is for another spine, or is not an overlay (T7.2).
    Import(String),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::NotFound(r) => write!(f, "{r} is not in the current text"),
            EditError::NoSuchMark { cluster, cp } => write!(f, "{cluster} has no mark {cp}"),
            EditError::Invalid(e) => write!(f, "invalid edit: {e}"),
            EditError::Import(e) => write!(f, "import: {e}"),
        }
    }
}

impl std::error::Error for EditError {}

/// The user's edit overlay with its undo and redo history.
#[derive(Clone, Debug)]
pub struct EditSession {
    overlay: EditOverlay,
    author: String,
    undo: Vec<EditOverlay>,
    redo: Vec<EditOverlay>,
    /// The next patch id is `u<next>`; never reused within a session.
    next: u64,
}

fn find_item<'a>(cur: &'a Resolved, r: &ItemRef) -> Option<&'a RItem> {
    cur.items.iter().find(|i| i.id == *r)
}

fn find_cluster<'a>(cur: &'a Resolved, r: &ClusterRef) -> Option<&'a RCluster> {
    find_item(cur, &r.item)?
        .clusters
        .iter()
        .find(|c| c.id == *r)
}

fn exists(cur: &Resolved, r: &Ref) -> bool {
    match r {
        Ref::Item(i) => find_item(cur, i).is_some(),
        Ref::Cluster(c) => find_cluster(cur, c).is_some(),
    }
}

/// True if `r` is `of`, lies inside it, or descends from it through inserts (`of+ins.k...`).
fn within(r: &Ref, of: &Ref) -> bool {
    let item_from = |a: &ItemRef, b: &ItemRef| {
        (a.surah, a.ayah, a.pos) == (b.surah, b.ayah, b.pos) && a.ins.starts_with(&b.ins)
    };
    match (r, of) {
        (Ref::Item(a), Ref::Item(b)) => item_from(a, b),
        (Ref::Cluster(a), Ref::Item(b)) => item_from(&a.item, b),
        (Ref::Cluster(a), Ref::Cluster(b)) => {
            a.item == b.item && a.n == b.n && a.ins.starts_with(&b.ins)
        }
        (Ref::Item(_), Ref::Cluster(_)) => false,
    }
}

impl EditSession {
    /// An empty session for the spine `spine`; `author` becomes the patches' `owner.id`.
    pub fn new(spine: impl Into<String>, author: impl Into<String>) -> Self {
        EditSession {
            overlay: EditOverlay::new(spine),
            author: author.into(),
            undo: Vec::new(),
            redo: Vec::new(),
            next: 1,
        }
    }

    pub fn overlay(&self) -> &EditOverlay {
        &self.overlay
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Restores the overlay before the last edit. False if there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.overlay, prev));
        true
    }

    /// Re-applies the last undone edit. False if there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.overlay, next));
        true
    }

    fn patch(&mut self, target: Ref, op: Op) -> Patch {
        let id = format!("u{}", self.next);
        self.next += 1;
        Patch {
            id,
            owner: Owner {
                level: OwnerLevel::User,
                id: self.author.clone(),
            },
            target,
            op,
            source: Source {
                kind: "user".into(),
                reference: "edit-session".into(),
            },
            status: Status::UserEdit,
            rule_id: None,
            when: None,
            single_source: false,
            difference_kind: None,
        }
    }

    /// Runs one edit on a copy of the overlay; on success it becomes the current overlay and
    /// the old one goes on the undo stack. A failed edit changes nothing.
    fn commit(
        &mut self,
        edit: impl FnOnce(&mut Self, &mut EditOverlay) -> Result<(), EditError>,
    ) -> Result<(), EditError> {
        let mut o = self.overlay.clone();
        let next = self.next;
        if let Err(e) = edit(self, &mut o) {
            self.next = next;
            return Err(e);
        }
        if o != self.overlay {
            self.undo.push(std::mem::replace(&mut self.overlay, o));
            self.redo.clear();
        }
        Ok(())
    }

    /// The user-layer insert patch that created `r`, if any.
    fn inserted_by(o: &EditOverlay, r: &Ref) -> Option<usize> {
        o.patches
            .iter()
            .position(|p| matches!(p.op, Op::Insert { .. }) && p.key() == *r)
    }

    fn upsert(o: &mut EditOverlay, p: Patch) -> Result<(), EditError> {
        o.upsert(p)
            .map(|_| ())
            .map_err(|e| EditError::Invalid(e.to_string()))
    }

    /// Inserts `node` (a cluster next to a cluster, or an item next to an item) and returns
    /// its new id, `<anchor>+ins.<seq>` with the smallest `seq` not in use.
    pub fn insert(
        &mut self,
        cur: &Resolved,
        anchor: Ref,
        side: Side,
        node: NewNode,
    ) -> Result<Ref, EditError> {
        if !exists(cur, &anchor) {
            return Err(EditError::NotFound(anchor));
        }
        let taken = |r: &Ref| exists(cur, r) || self.overlay.patches.iter().any(|p| p.key() == *r);
        let seq = (1..)
            .find(|s| !taken(&anchor.inserted(*s)))
            .expect("a free seq");
        let new = anchor.inserted(seq);
        self.commit(|s, o| {
            let p = s.patch(anchor, Op::Insert { side, seq, node });
            Self::upsert(o, p)
        })?;
        Ok(new)
    }

    /// Deletes an item or a cluster. Deleting something this session inserted removes its
    /// insert (and the user's edits on and inside it) instead of writing a delete.
    pub fn delete(&mut self, cur: &Resolved, target: Ref) -> Result<(), EditError> {
        if !exists(cur, &target) {
            return Err(EditError::NotFound(target));
        }
        self.commit(|s, o| {
            let own = Self::inserted_by(o, &target).is_some();
            // Edits on the target or inside it would be orphans, or (inside a deleted item)
            // an overlap error; an insert next to a deleted item keeps its place (D12).
            o.patches.retain(|p| {
                let k = p.key();
                let on_or_in = k == target || k.is_inside(&target) || (own && within(&k, &target));
                !on_or_in
            });
            if own {
                Ok(())
            } else {
                let p = s.patch(target, Op::Delete);
                Self::upsert(o, p)
            }
        })
    }

    /// Sets a cluster's kind and codepoints.
    pub fn replace_cluster(
        &mut self,
        cur: &Resolved,
        target: ClusterRef,
        cluster: NewCluster,
    ) -> Result<(), EditError> {
        let r = Ref::Cluster(target);
        if !exists(cur, &r) {
            return Err(EditError::NotFound(r));
        }
        if cluster.cps.is_empty() {
            return Err(EditError::Invalid(
                "a cluster needs at least one codepoint".into(),
            ));
        }
        self.commit(|s, o| {
            if let Some(i) = Self::inserted_by(o, &r) {
                if let Op::Insert { node, .. } = &mut o.patches[i].op {
                    *node = NewNode::Cluster(cluster);
                }
                return o.patches[i]
                    .validate()
                    .map_err(|e| EditError::Invalid(e.to_string()));
            }
            if let Some(i) = Self::inserted_by(o, &Ref::Item(r.item().clone())) {
                // A cluster of an item this session inserted: `<item-ins-id>:<k>`.
                if let Op::Insert {
                    node: NewNode::Item(item),
                    ..
                } = &mut o.patches[i].op
                {
                    let ClusterRef { n, ins, .. } = match &r {
                        Ref::Cluster(c) => c,
                        Ref::Item(_) => unreachable!(),
                    };
                    if ins.is_empty() {
                        let k = usize::try_from(*n).expect("index fits") - 1;
                        item.clusters[k] = cluster;
                        return o.patches[i]
                            .validate()
                            .map_err(|e| EditError::Invalid(e.to_string()));
                    }
                }
            }
            let p = s.patch(r, Op::ReplaceCluster(cluster));
            Self::upsert(o, p)
        })
    }

    fn current(cur: &Resolved, target: &ClusterRef) -> Result<(ClusterKind, Vec<char>), EditError> {
        find_cluster(cur, target)
            .map(|c| (c.kind, c.cps.0.clone()))
            .ok_or_else(|| EditError::NotFound(Ref::Cluster(target.clone())))
    }

    /// Adds a mark to a cluster: at mark index `at` (0 = right after the base), or after the
    /// existing marks. Written as the cluster's exact new codepoints (D53).
    pub fn add_mark(
        &mut self,
        cur: &Resolved,
        target: ClusterRef,
        cp: Cp,
        at: Option<u32>,
    ) -> Result<(), EditError> {
        let (kind, mut cps) = Self::current(cur, &target)?;
        let i = at.map_or(cps.len(), |a| {
            (usize::try_from(a).expect("index fits") + 1).min(cps.len())
        });
        cps.insert(i, cp.0);
        self.replace_cluster(
            cur,
            target,
            NewCluster {
                kind,
                cps: Cps(cps),
            },
        )
    }

    /// Removes the first occurrence of a mark (a codepoint after the base) from a cluster.
    pub fn remove_mark(
        &mut self,
        cur: &Resolved,
        target: ClusterRef,
        cp: Cp,
    ) -> Result<(), EditError> {
        let (kind, mut cps) = Self::current(cur, &target)?;
        let Some(m) = cps.iter().skip(1).position(|c| *c == cp.0) else {
            return Err(EditError::NoSuchMark {
                cluster: target,
                cp,
            });
        };
        cps.remove(m + 1);
        self.replace_cluster(
            cur,
            target,
            NewCluster {
                kind,
                cps: Cps(cps),
            },
        )
    }

    fn set_attr(
        &mut self,
        cur: &Resolved,
        target: Ref,
        set: impl FnOnce(&mut AttrChange),
    ) -> Result<(), EditError> {
        if !exists(cur, &target) {
            return Err(EditError::NotFound(target));
        }
        self.commit(|s, o| {
            let slot = (target.clone(), true);
            let old = o.patches.iter().position(|p| p.slot() == slot);
            let mut change = match old.map(|i| &o.patches[i].op) {
                Some(Op::SetAttr(c)) => c.clone(),
                _ => AttrChange::default(),
            };
            set(&mut change);
            if let Some(i) = old {
                o.patches.remove(i);
            }
            if change == AttrChange::default() {
                return Ok(());
            }
            let p = s.patch(target, Op::SetAttr(change));
            Self::upsert(o, p)
        })
    }

    /// Sets the extra advance (1/1000 em, D16) after a word or a cluster; `None` drops the
    /// user's value, so the lower layers' value (if any) applies again.
    pub fn set_width(
        &mut self,
        cur: &Resolved,
        target: Ref,
        value: Option<i32>,
    ) -> Result<(), EditError> {
        self.set_attr(cur, target, |c| c.width_em_delta = value)
    }

    /// Sets a letter's stretch (1/1000 em, D16); `None` drops the user's value.
    pub fn set_stretch(
        &mut self,
        cur: &Resolved,
        target: ClusterRef,
        value: Option<i32>,
    ) -> Result<(), EditError> {
        self.set_attr(cur, Ref::Cluster(target), |c| c.stretch = value)
    }
}
