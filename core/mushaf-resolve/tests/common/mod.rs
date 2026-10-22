//! Shared helpers: the synthetic fixture spine (Latin letters only, rule 1) and a compact
//! patch builder that goes through the same YAML parser as patch files.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use mushaf_model::{DataSet, Item, Patch, PatchFile, Spine, from_yaml};
use mushaf_resolve::Resolved;

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

pub fn synthetic() -> DataSet {
    mushaf_db::load_dir(&repo().join("tests/fixtures/synthetic")).unwrap()
}

pub fn spine() -> Spine {
    synthetic().spines.remove("test/v1").unwrap()
}

/// A patch: `level` is qiraah | riwayah | tariq | user; `extra` holds further YAML fields
/// (payload, when), indented by four spaces.
pub fn patch(id: &str, level: &str, op: &str, target: &str, extra: &str) -> Patch {
    let status = if level == "user" {
        "user-edit"
    } else {
        "raw-derived"
    };
    let y = format!(
        "spine: \"test/v1\"\npatches:\n  - id: \"{id}\"\n    owner: {{level: \"{level}\", id: \"o\"}}\n    \
         op: \"{op}\"\n    target: \"{target}\"\n{extra}    source: {{kind: \"test\", ref: \"unit\"}}\n    \
         status: \"{status}\"\n"
    );
    from_yaml::<PatchFile>(&y)
        .unwrap_or_else(|e| panic!("{e}\n{y}"))
        .patches
        .remove(0)
}

pub fn payload(flow: &str) -> String {
    format!("    payload: {flow}\n")
}

/// The text of one resolved item, or None if it is not in the output.
pub fn item_text(r: &Resolved, id: &str) -> Option<String> {
    r.items
        .iter()
        .find(|i| i.id.to_string() == id)
        .map(|i| i.text())
}

pub fn ids(r: &Resolved) -> Vec<String> {
    r.items.iter().map(|i| i.id.to_string()).collect()
}

/// The spine items a resolution reproduces when nothing changed them.
pub fn as_spine_items(r: &Resolved) -> Vec<Item> {
    r.items
        .iter()
        .map(|i| Item {
            id: i.id.clone(),
            kind: i.kind,
            sep_before: i.sep_before.clone(),
            sep_after: i.sep_after.clone(),
            clusters: i
                .clusters
                .iter()
                .map(|c| mushaf_model::Cluster {
                    id: c.id.clone(),
                    kind: c.kind,
                    cps: c.cps.clone(),
                })
                .collect(),
        })
        .collect()
}
