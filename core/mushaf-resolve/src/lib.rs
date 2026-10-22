//! Patch engine, option groups and user edit overlay (PROJECT_PLAN §3.2, §3.3; D9, D12).
//!
//! Pure and deterministic: the same spine, patches and options always give the same
//! `Resolved`, byte for byte when serialized. Free mode never refuses an edit (rule 11):
//! problems with individual patches are reported, never silently dropped; `Mode::Data` turns
//! any report into an error.

use std::fmt;

use mushaf_model::{Attrs, ClusterKind, ClusterRef, Cps, ItemKind, ItemRef, Layer, Ref};
use serde::Serialize;

mod engine;

pub use engine::resolve_patches;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Data builds: any report entry is an error.
    Data,
    /// Free-edit mode: report entries are returned with the result.
    Free,
}

/// Where a resolved item comes from, for line mapping (T2.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// The spine item at this position.
    Spine { pos: usize },
    /// An inserted item; `after_spine` is the nearest spine node before it in reading order
    /// (deleted or not), or None if it precedes the first spine item.
    Inserted { after_spine: Option<usize> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RCluster {
    pub id: ClusterRef,
    pub kind: ClusterKind,
    pub cps: Cps,
    #[serde(skip_serializing_if = "Attrs::is_empty")]
    pub attrs: Attrs,
    /// Ids of the patches that created or changed this cluster, lowest layer first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RItem {
    pub id: ItemRef,
    pub kind: ItemKind,
    #[serde(skip_serializing_if = "Cps::is_empty")]
    pub sep_before: Cps,
    pub sep_after: Cps,
    #[serde(skip_serializing_if = "Attrs::is_empty")]
    pub attrs: Attrs,
    pub clusters: Vec<RCluster>,
    pub origin: Origin,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<String>,
}

impl RItem {
    /// The item's text exactly as stored: separators and cluster codepoints.
    pub fn text(&self) -> String {
        let mut s = self.sep_before.text();
        for c in &self.clusters {
            s.extend(c.cps.0.iter());
        }
        s + &self.sep_after.text()
    }
}

/// A problem with one patch (or the result), reported and never silently dropped (D9, D12).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Diagnostic {
    /// The target (or an insert's anchor) was deleted by a lower layer. A modification is not
    /// applied; an insert is placed at the deleted anchor's position (`applied` if visible).
    Orphan {
        patch: String,
        target: Ref,
        applied: bool,
    },
    /// The target does not exist in the spine or among inserted nodes. Not applied.
    Unresolved { patch: String, target: Ref },
    /// An insert would create an id a lower layer already created. Not applied.
    IdCollision { patch: String, id: Ref },
    /// Applied, but part of it changed nothing or was adjusted (reason given).
    Ineffective {
        patch: String,
        target: Ref,
        reason: String,
    },
    /// An item that is not deleted has no clusters left.
    EmptyItem { item: ItemRef },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Resolved {
    pub items: Vec<RItem>,
    /// Sorted, so it does not depend on patch order.
    pub report: Vec<Diagnostic>,
}

impl Resolved {
    pub fn text(&self) -> String {
        self.items.iter().map(RItem::text).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveError {
    /// A patch built in code breaks the rules parsing enforces.
    InvalidPatch(String),
    /// Two patches in the stack share an id.
    DuplicatePatchId(String),
    /// Two patches claim the same key in one layer (D9).
    Conflict {
        layer: Layer,
        key: Box<Ref>,
        patches: Vec<String>,
    },
    /// A patch inside an item that its own layer deletes.
    Overlap {
        layer: Layer,
        key: Box<Ref>,
        deleting: String,
        patch: String,
    },
    /// `Mode::Data` and the report is not empty.
    Strict(Vec<Diagnostic>),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::InvalidPatch(e) => write!(f, "invalid patch: {e}"),
            ResolveError::DuplicatePatchId(id) => write!(f, "patch id {id:?} occurs twice"),
            ResolveError::Conflict {
                layer,
                key,
                patches,
            } => {
                write!(f, "conflict in layer {layer:?} on {key}: {patches:?}")
            }
            ResolveError::Overlap {
                layer,
                key,
                deleting,
                patch,
            } => write!(
                f,
                "patch {patch:?} on {key} is inside an item that {deleting:?} deletes in the same \
                 layer ({layer:?})"
            ),
            ResolveError::Strict(report) => {
                write!(f, "data mode: {} report entries: {report:?}", report.len())
            }
        }
    }
}

impl std::error::Error for ResolveError {}
