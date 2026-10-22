//! The spine: items (words, ayah-end markers, hizb signs) made of clusters (§3.1, D11).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ClusterRef, Cps, Error, ItemRef};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Word,
    AyahEnd,
    Hizb,
}

/// Cluster kinds produced by the clusterizer (`data/pipeline/clusterize.py`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterKind {
    Letter,
    Sign,
    AyahNumber,
    Digit,
    Control,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
    pub id: ClusterRef,
    pub kind: ClusterKind,
    pub cps: Cps,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: ItemRef,
    pub kind: ItemKind,
    /// Only set for listed source anomalies (D11, G-026).
    #[serde(default, skip_serializing_if = "Cps::is_empty")]
    pub sep_before: Cps,
    pub sep_after: Cps,
    pub clusters: Vec<Cluster>,
}

/// Per-cluster or per-item presentation attributes (§3.1). Integers only (D10, D12):
/// `width_em_delta` in 1/1000 em, `stretch` in 1/1000 (meaning fixed in T3.2).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attrs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_em_delta: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stretch: Option<i32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

impl Attrs {
    pub fn is_empty(&self) -> bool {
        *self == Attrs::default()
    }
}

/// A validated spine. Items are in reading order and their ids strictly increase, so an id
/// is found by binary search; inserted ids never occur in a spine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spine {
    /// `<name>/v<version>`, e.g. `hafs/v1`; patch files name the spine they target.
    pub id: String,
    pub script_id: String,
    pub version: u32,
    pub items: Vec<Item>,
}

fn key(r: &ItemRef) -> (u32, u32, u32) {
    (r.surah, r.ayah, r.pos)
}

impl Spine {
    pub fn new(
        id: String,
        script_id: String,
        version: u32,
        items: Vec<Item>,
    ) -> Result<Self, Error> {
        if !id.ends_with(&format!("/v{version}")) {
            return Err(Error::new(format!(
                "spine id {id:?} does not end with /v{version}"
            )));
        }
        for (i, item) in items.iter().enumerate() {
            if !item.id.is_spine() {
                return Err(Error::new(format!(
                    "spine item {} has an insert id",
                    item.id
                )));
            }
            if i > 0 && key(&items[i - 1].id) >= key(&item.id) {
                return Err(Error::new(format!(
                    "spine item {} is out of order",
                    item.id
                )));
            }
            if item.clusters.is_empty() {
                return Err(Error::new(format!(
                    "spine item {} has no clusters",
                    item.id
                )));
            }
            for (k, c) in item.clusters.iter().enumerate() {
                let want = item.id.cluster(k as u32 + 1);
                if c.id != want {
                    return Err(Error::new(format!("cluster {} should be {want}", c.id)));
                }
                if c.cps.is_empty() {
                    return Err(Error::new(format!("cluster {} is empty", c.id)));
                }
            }
        }
        Ok(Spine {
            id,
            script_id,
            version,
            items,
        })
    }

    /// Position of a spine item, or None (also for inserted ids).
    pub fn position(&self, r: &ItemRef) -> Option<usize> {
        if !r.is_spine() {
            return None;
        }
        self.items
            .binary_search_by_key(&key(r), |it| key(&it.id))
            .ok()
    }

    pub fn item(&self, r: &ItemRef) -> Option<&Item> {
        self.position(r).map(|i| &self.items[i])
    }

    pub fn cluster(&self, r: &ClusterRef) -> Option<&Cluster> {
        if !r.ins.is_empty() {
            return None;
        }
        let item = self.item(&r.item)?;
        item.clusters.get((r.n as usize).checked_sub(1)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, letters: &[&str]) -> Item {
        let id: ItemRef = id.parse().unwrap();
        Item {
            clusters: letters
                .iter()
                .enumerate()
                .map(|(k, cps)| Cluster {
                    id: id.cluster(k as u32 + 1),
                    kind: ClusterKind::Letter,
                    cps: cps.parse().unwrap(),
                })
                .collect(),
            id,
            kind: ItemKind::Word,
            sep_before: Cps::default(),
            sep_after: "U+0020".parse().unwrap(),
        }
    }

    #[test]
    fn validates_order_and_cluster_ids() {
        let a = item("1:1:1", &["U+0061", "U+0062 U+0301"]);
        let b = item("1:1:2", &["U+0063"]);
        let s = Spine::new("t/v1".into(), "test".into(), 1, vec![a.clone(), b.clone()]).unwrap();
        assert_eq!(s.position(&"1:1:2".parse().unwrap()), Some(1));
        assert_eq!(s.position(&"1:1:2+ins.1".parse().unwrap()), None);
        assert_eq!(
            s.cluster(&"1:1:1:2".parse().unwrap()).unwrap().cps.text(),
            "b\u{301}"
        );
        assert!(s.cluster(&"1:1:1:3".parse().unwrap()).is_none());
        assert!(Spine::new("t/v1".into(), "test".into(), 1, vec![b, a.clone()]).is_err());
        assert!(Spine::new("t/v2".into(), "test".into(), 1, vec![a.clone()]).is_err());
        let mut bad = a;
        bad.clusters[1].id = "1:1:1:3".parse().unwrap();
        assert!(Spine::new("t/v1".into(), "test".into(), 1, vec![bad]).is_err());
    }

    #[test]
    fn item_json_matches_the_tracked_form() {
        let line = r#"{"id":"2:6:1","kind":"word","sep_before":"U+0020","sep_after":"U+0020","clusters":[{"id":"2:6:1:1","kind":"letter","cps":"U+0061"}]}"#;
        let it: Item = serde_json::from_str(line).unwrap();
        assert_eq!(serde_json::to_string(&it).unwrap(), line);
        assert!(
            serde_json::from_str::<Item>(&line.replace("\"kind\":\"word\"", "\"kind\":\"verse\""))
                .is_err()
        );
        assert!(
            serde_json::from_str::<Item>(
                &line.replace("\"sep_after\"", "\"extra\":1,\"sep_after\"")
            )
            .is_err()
        );
    }

    #[test]
    fn attrs_reject_floats() {
        assert!(serde_json::from_str::<Attrs>(r#"{"width_em_delta":0.5}"#).is_err());
        let a: Attrs = serde_json::from_str(r#"{"width_em_delta":-250}"#).unwrap();
        assert_eq!(a.width_em_delta, Some(-250));
    }
}
