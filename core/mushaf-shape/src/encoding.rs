//! KFGQPC -> font encoding conversion (D2, D14).
//!
//! The table is data derived by `data/pipeline/dk_encoding.py` from pinned sources and tracked
//! as `data/encodings/<spine>.kfgqpc-to-<font>.json`. This module only applies it, exactly as
//! the pipeline does: at each position the longest matching sequence row wins, else the
//! cluster row; a cluster with no row is an error, never passed through unchanged.

use std::collections::BTreeMap;
use std::ops::Range;

use mushaf_model::{Cps, Error};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: u32,
    spine: String,
    from: String,
    to: String,
    #[allow(dead_code)]
    provenance: serde_json::Value,
    converted_spine: Digest,
    lossy_items: Vec<String>,
    rows: Vec<WireRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRow {
    src: Vec<Cps>,
    dst: Cps,
    #[allow(dead_code)]
    count: u64,
    kind: String,
    lossy: bool,
}

/// sha256 over `id TAB converted cps LF` for every spine item, computed by the pipeline.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Digest {
    pub items: usize,
    pub sha256: String,
}

#[derive(Clone, Debug)]
struct Target {
    dst: Vec<char>,
    lossy: bool,
}

#[derive(Clone, Debug)]
pub struct EncodingTable {
    pub spine: String,
    /// Target encoding, e.g. `digitalkhatt-madina`.
    pub to: String,
    pub converted_spine: Digest,
    /// Spine items whose conversion uses a lossy row (G-040), as the pipeline lists them.
    pub lossy_items: Vec<String>,
    clusters: BTreeMap<Vec<char>, Target>,
    sequences: BTreeMap<Vec<Vec<char>>, Target>,
    max_seq: usize,
}

/// Which source clusters produced which characters of the converted text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub clusters: Range<usize>,
    pub chars: Range<usize>,
    /// The row merges this form with another (D14): the font shows the other's form.
    pub lossy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Converted {
    pub chars: Vec<char>,
    pub spans: Vec<Span>,
}

/// A cluster the table has no row for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unmapped {
    pub cluster: usize,
    pub cps: Cps,
}

impl EncodingTable {
    pub fn from_json(text: &str) -> Result<Self, Error> {
        let wire: Wire = mushaf_model::from_json(text)?;
        if wire.schema != 1 || wire.from != "kfgqpc" {
            return Err(Error::new("unsupported encoding table"));
        }
        let mut clusters = BTreeMap::new();
        let mut sequences = BTreeMap::new();
        for r in wire.rows {
            let target = Target {
                dst: r.dst.0,
                lossy: r.lossy,
            };
            let src: Vec<Vec<char>> = r.src.into_iter().map(|c| c.0).collect();
            let dup = match (r.kind.as_str(), src.len()) {
                ("cluster" | "owner", 1) => clusters.insert(src[0].clone(), target).is_some(),
                ("sequence", n) if n >= 1 => sequences.insert(src, target).is_some(),
                (kind, _) => return Err(Error::new(format!("bad row kind {kind:?}"))),
            };
            if dup {
                return Err(Error::new("duplicate row in encoding table"));
            }
        }
        let max_seq = sequences.keys().map(Vec::len).max().unwrap_or(0);
        Ok(EncodingTable {
            spine: wire.spine,
            to: wire.to,
            converted_spine: wire.converted_spine,
            lossy_items: wire.lossy_items,
            clusters,
            sequences,
            max_seq,
        })
    }

    /// Convert one item's clusters (KFGQPC codepoints) to the font's encoding.
    pub fn convert(&self, clusters: &[&[char]]) -> Result<Converted, Unmapped> {
        let mut out = Converted {
            chars: Vec::new(),
            spans: Vec::new(),
        };
        let mut i = 0;
        while i < clusters.len() {
            let (n, target) = self.lookup(&clusters[i..]).ok_or_else(|| Unmapped {
                cluster: i,
                cps: Cps(clusters[i].to_vec()),
            })?;
            let start = out.chars.len();
            out.chars.extend_from_slice(&target.dst);
            out.spans.push(Span {
                clusters: i..i + n,
                chars: start..out.chars.len(),
                lossy: target.lossy,
            });
            i += n;
        }
        Ok(out)
    }

    fn lookup(&self, rest: &[&[char]]) -> Option<(usize, &Target)> {
        for n in (1..=self.max_seq.min(rest.len())).rev() {
            let key: Vec<Vec<char>> = rest[..n].iter().map(|c| c.to_vec()).collect();
            if let Some(t) = self.sequences.get(&key) {
                return Some((n, t));
            }
        }
        self.clusters.get(rest[0]).map(|t| (1, t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic Latin "clusters": test fixtures, not Quranic text.
    const TABLE: &str = r#"{"schema": 1, "spine": "t/v1", "from": "kfgqpc", "to": "f",
      "provenance": {}, "converted_spine": {"items": 0, "sha256": ""}, "lossy_items": [],
      "rows": [
        {"src": ["U+0061"], "dst": "U+0061", "count": 1, "kind": "cluster", "lossy": false},
        {"src": ["U+0062"], "dst": "U+0042", "count": 1, "kind": "cluster", "lossy": true},
        {"src": ["U+0071"], "dst": "U+0051 U+0052", "count": 1, "kind": "sequence", "lossy": false},
        {"src": ["U+0062", "U+0063"], "dst": "U+0058", "count": 1, "kind": "sequence", "lossy": false}
      ]}"#;

    fn conv(t: &EncodingTable, s: &str) -> Result<Converted, Unmapped> {
        let chars: Vec<Vec<char>> = s.split(' ').map(|c| c.chars().collect()).collect();
        let refs: Vec<&[char]> = chars.iter().map(Vec::as_slice).collect();
        t.convert(&refs)
    }

    #[test]
    fn longest_sequence_wins_then_cluster_rows() {
        let t = EncodingTable::from_json(TABLE).unwrap();
        let c = conv(&t, "a q b c b").unwrap();
        assert_eq!(c.chars.iter().collect::<String>(), "aQRXB");
        assert_eq!(
            c.spans,
            vec![
                Span {
                    clusters: 0..1,
                    chars: 0..1,
                    lossy: false
                },
                Span {
                    clusters: 1..2,
                    chars: 1..3,
                    lossy: false
                },
                Span {
                    clusters: 2..4,
                    chars: 3..4,
                    lossy: false
                },
                Span {
                    clusters: 4..5,
                    chars: 4..5,
                    lossy: true
                },
            ]
        );
    }

    #[test]
    fn a_cluster_without_a_row_is_an_error() {
        let t = EncodingTable::from_json(TABLE).unwrap();
        let err = conv(&t, "a z").unwrap_err();
        assert_eq!(err.cluster, 1);
    }

    #[test]
    fn unknown_fields_and_duplicate_rows_are_rejected() {
        assert!(
            EncodingTable::from_json(&TABLE.replace("\"schema\"", "\"x\": 1, \"schema\"")).is_err()
        );
        let dup = TABLE.replace(
            "\"U+0062\"], \"dst\": \"U+0042\"",
            "\"U+0061\"], \"dst\": \"U+0042\"",
        );
        assert!(EncodingTable::from_json(&dup).is_err());
    }
}
