//! KFGQPC -> font encoding conversion (D2, D14).
//!
//! The table is data derived by `data/pipeline/dk_encoding.py` from pinned sources and tracked
//! as `data/encodings/<spine>.kfgqpc-to-<font>.json`. This module only applies it, exactly as
//! the pipeline does: at each position the longest matching sequence row wins, else the
//! cluster row; a cluster with no row is an error, never passed through unchanged.
//!
//! A font made for KFGQPC's own encoding (the KFGQPC Warsh and Qālūn fonts, D2) uses the
//! identity table instead: every cluster is drawn exactly as stored (T5.3).

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
    word_rows: Vec<WireWord>,
    rows: Vec<WireRow>,
}

/// An owner word row (D18): a spine item rendered as DigitalKhatt's own word.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireWord {
    item: String,
    src: Vec<Cps>,
    dst: Cps,
    #[allow(dead_code)]
    decision: String,
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

/// `data/encodings/<spine>.riwayat-rows.json` (T5.3, D28): rows for cluster forms that occur only
/// outside Ḥafṣ, each from an owner-approved group or an owner row.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtraWire {
    schema: u32,
    spine: String,
    from: String,
    to: String,
    #[allow(dead_code)]
    note: String,
    rows: Vec<ExtraRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtraRow {
    src: Vec<Cps>,
    dst: Cps,
    kind: String,
    #[allow(dead_code)]
    groups: Vec<String>,
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
    /// Owner-approved drawing adjustments (D47), char indices relative to `dst`.
    adjust: Vec<Adjust>,
}

/// An owner-approved drawing adjustment of a converted form (D47), in font units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum Adjust {
    /// Extra advance after character `at`.
    Gap { at: usize, fu: i32 },
    /// Move the glyph of codepoint `cp` (nominal glyph or a variant the font substitutes) in
    /// the cluster of character `at` by (`dx`, `dy`), y up.
    Shift {
        at: usize,
        cp: Cps1,
        dx: i32,
        dy: i32,
    },
}

/// One codepoint written `U+XXXX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Cps")]
pub struct Cps1(pub char);

impl TryFrom<Cps> for Cps1 {
    type Error = String;
    fn try_from(c: Cps) -> Result<Self, String> {
        match c.0.as_slice() {
            [ch] => Ok(Cps1(*ch)),
            _ => Err(format!("{c}: expected one codepoint")),
        }
    }
}

impl Adjust {
    fn at(&self) -> usize {
        match *self {
            Adjust::Gap { at, .. } | Adjust::Shift { at, .. } => at,
        }
    }

    fn offset(self, by: usize) -> Self {
        match self {
            Adjust::Gap { at, fu } => Adjust::Gap { at: at + by, fu },
            Adjust::Shift { at, cp, dx, dy } => Adjust::Shift {
                at: at + by,
                cp,
                dx,
                dy,
            },
        }
    }
}

/// `data/encodings/<spine>.hijazi-forms.json` (D47): owner-approved forms for a Maghribi
/// riwāyah drawn in the Hijazi style. Unlike the extra rows they may replace a row of the table.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormsWire {
    schema: u32,
    spine: String,
    from: String,
    to: String,
    #[allow(dead_code)]
    note: String,
    rows: Vec<FormRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormRow {
    src: Vec<Cps>,
    dst: Cps,
    #[serde(default)]
    adjust: Vec<Adjust>,
    #[allow(dead_code)]
    decision: String,
    #[allow(dead_code)]
    #[serde(default)]
    note: String,
}

#[derive(Clone, Debug)]
pub struct EncodingTable {
    pub spine: String,
    /// Target encoding, e.g. `digitalkhatt-madina`.
    pub to: String,
    pub converted_spine: Digest,
    /// Spine items whose conversion uses a lossy row (G-040), as the pipeline lists them.
    pub lossy_items: Vec<String>,
    /// Owner word rows by item id: (KFGQPC clusters, font-encoded word) (D18).
    word_rows: BTreeMap<String, (Vec<Vec<char>>, Vec<char>)>,
    clusters: BTreeMap<Vec<char>, Target>,
    sequences: BTreeMap<Vec<Vec<char>>, Target>,
    max_seq: usize,
    identity: bool,
    /// Style conversion (T6.7, D44): codepoint substitutions applied to every cluster before
    /// the table, at render time only.
    premap: BTreeMap<char, char>,
}

/// `data/encodings/style-conversion.json` (T6.7, D44): owner-confirmed codepoint rows between
/// the mark conventions of two display styles.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StyleWire {
    schema: u32,
    #[allow(dead_code)]
    note: String,
    rows: Vec<StyleRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StyleRow {
    from_style: String,
    to_style: String,
    from: Cps,
    to: Cps,
    #[allow(dead_code)]
    evidence: String,
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
    /// Drawing adjustments (D47), char indices into `chars`.
    pub adjust: Vec<Adjust>,
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
                adjust: Vec::new(),
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
            word_rows: wire
                .word_rows
                .into_iter()
                .map(|w| (w.item, (w.src.into_iter().map(|c| c.0).collect(), w.dst.0)))
                .collect(),
            clusters,
            sequences,
            max_seq,
            identity: false,
            premap: BTreeMap::new(),
        })
    }

    /// Adds the approved rows for the other riwāyāt (T5.3, D28). A row for a form the table
    /// already converts is an error: these rows only cover forms Ḥafṣ does not have.
    pub fn extend_from_json(&mut self, text: &str) -> Result<(), Error> {
        let wire: ExtraWire = mushaf_model::from_json(text)?;
        if wire.schema != 1
            || wire.from != "kfgqpc"
            || wire.to != self.to
            || wire.spine != self.spine
        {
            return Err(Error::new("extra rows are for another table"));
        }
        for r in wire.rows {
            let target = Target {
                dst: r.dst.0,
                lossy: false,
                adjust: Vec::new(),
            };
            let src: Vec<Vec<char>> = r.src.into_iter().map(|c| c.0).collect();
            let dup = match (r.kind.as_str(), src.len()) {
                ("cluster" | "owner", 1) => self.clusters.insert(src[0].clone(), target).is_some(),
                ("sequence", n) if n >= 1 => self.sequences.insert(src, target).is_some(),
                (kind, _) => return Err(Error::new(format!("bad extra row kind {kind:?}"))),
            };
            if dup {
                return Err(Error::new("an extra row repeats a row of the table"));
            }
        }
        self.max_seq = self.sequences.keys().map(Vec::len).max().unwrap_or(0);
        Ok(())
    }

    /// Adds the owner-approved Hijazi forms of the Maghribi riwāyāt (D47). A row may replace a
    /// row of the table; a form listed twice, or an adjustment outside its form, is an error.
    pub fn extend_forms_from_json(&mut self, text: &str) -> Result<(), Error> {
        let wire: FormsWire = mushaf_model::from_json(text)?;
        if wire.schema != 1
            || wire.from != "kfgqpc"
            || wire.to != self.to
            || wire.spine != self.spine
        {
            return Err(Error::new("hijazi forms are for another table"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for r in wire.rows {
            if let Some(a) = r.adjust.iter().find(|a| a.at() >= r.dst.0.len()) {
                return Err(Error::new(format!(
                    "hijazi form {}: {a:?} outside the form",
                    r.dst
                )));
            }
            let src: Vec<Vec<char>> = r.src.into_iter().map(|c| c.0).collect();
            if !seen.insert(src.clone()) {
                return Err(Error::new("a hijazi form is listed twice"));
            }
            let target = Target {
                dst: r.dst.0,
                lossy: false,
                adjust: r.adjust,
            };
            if src.len() == 1 {
                self.clusters.insert(src[0].clone(), target);
            } else {
                self.sequences.insert(src, target);
            }
        }
        self.max_seq = self.sequences.keys().map(Vec::len).max().unwrap_or(0);
        Ok(())
    }

    /// The table of a font that reads KFGQPC's encoding: each cluster converts to itself.
    pub fn identity(spine: &str) -> Self {
        EncodingTable {
            spine: spine.into(),
            to: "kfgqpc".into(),
            converted_spine: Digest {
                items: 0,
                sha256: String::new(),
            },
            lossy_items: Vec::new(),
            word_rows: BTreeMap::new(),
            clusters: BTreeMap::new(),
            sequences: BTreeMap::new(),
            max_seq: 0,
            identity: true,
            premap: BTreeMap::new(),
        }
    }

    /// Adds the style conversion rows from `from_style` to `to_style` (T6.7, D44). Each row maps
    /// one codepoint to one codepoint; a codepoint mapped twice is an error.
    pub fn with_style_conversion(
        mut self,
        text: &str,
        from_style: &str,
        to_style: &str,
    ) -> Result<Self, Error> {
        let wire: StyleWire = mushaf_model::from_json(text)?;
        if wire.schema != 1 {
            return Err(Error::new("style conversion: unknown schema"));
        }
        for r in wire.rows {
            if r.from_style != from_style || r.to_style != to_style {
                continue;
            }
            let ([a], [b]) = (r.from.0.as_slice(), r.to.0.as_slice()) else {
                return Err(Error::new("style conversion rows map one codepoint to one"));
            };
            if self.premap.insert(*a, *b).is_some() {
                return Err(Error::new(format!(
                    "style conversion: {} mapped twice",
                    r.from
                )));
            }
        }
        Ok(self)
    }

    /// The style substitutions applied by `convert`.
    pub fn style_conversion(&self) -> &BTreeMap<char, char> {
        &self.premap
    }

    /// Convert a spine item: its owner word row (D18) when the table lists this item with
    /// exactly these clusters, else cluster by cluster.
    pub fn convert_item(&self, item: &str, clusters: &[&[char]]) -> Result<Converted, Unmapped> {
        if let Some((src, dst)) = self.word_rows.get(item)
            && src.len() == clusters.len()
            && src.iter().zip(clusters).all(|(a, b)| a.as_slice() == *b)
        {
            return Ok(Converted {
                adjust: Vec::new(),
                chars: dst.clone(),
                spans: vec![Span {
                    clusters: 0..clusters.len(),
                    chars: 0..dst.len(),
                    lossy: false,
                }],
            });
        }
        self.convert(clusters)
    }

    /// Convert one item's clusters (KFGQPC codepoints) to the font's encoding, after the style
    /// conversion if any.
    pub fn convert(&self, clusters: &[&[char]]) -> Result<Converted, Unmapped> {
        if !self.premap.is_empty() {
            let mapped: Vec<Vec<char>> = clusters
                .iter()
                .map(|c| {
                    c.iter()
                        .map(|ch| *self.premap.get(ch).unwrap_or(ch))
                        .collect()
                })
                .collect();
            let refs: Vec<&[char]> = mapped.iter().map(Vec::as_slice).collect();
            return self.convert_mapped(&refs);
        }
        self.convert_mapped(clusters)
    }

    fn convert_mapped(&self, clusters: &[&[char]]) -> Result<Converted, Unmapped> {
        let mut out = Converted {
            chars: Vec::new(),
            spans: Vec::new(),
            adjust: Vec::new(),
        };
        if self.identity {
            for (i, c) in clusters.iter().enumerate() {
                let start = out.chars.len();
                out.chars.extend_from_slice(c);
                out.spans.push(Span {
                    clusters: i..i + 1,
                    chars: start..out.chars.len(),
                    lossy: false,
                });
            }
            return Ok(out);
        }
        let mut i = 0;
        while i < clusters.len() {
            let (n, target) = self.lookup(&clusters[i..]).ok_or_else(|| Unmapped {
                cluster: i,
                cps: Cps(clusters[i].to_vec()),
            })?;
            let start = out.chars.len();
            out.chars.extend_from_slice(&target.dst);
            out.adjust
                .extend(target.adjust.iter().map(|a| a.offset(start)));
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
      "word_rows": [{"item": "1:1:1", "src": ["U+0061", "U+007A"], "dst": "U+0057", "decision": "test"}],
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
    fn an_owner_word_row_applies_to_its_item_and_clusters_only() {
        let t = EncodingTable::from_json(TABLE).unwrap();
        let a: Vec<char> = vec!['a'];
        let z: Vec<char> = vec!['z'];
        let cl = [a.as_slice(), z.as_slice()];
        let c = t.convert_item("1:1:1", &cl).unwrap();
        assert_eq!(c.chars, vec!['W']);
        assert_eq!(c.spans.len(), 1);
        assert!(
            t.convert_item("1:1:2", &cl).is_err(),
            "another item converts cluster by cluster"
        );
        assert!(t.convert_item("1:1:1", &cl[..1]).is_ok());
    }

    #[test]
    fn extra_rows_add_forms_but_never_replace_rows() {
        let mut t = EncodingTable::from_json(TABLE).unwrap();
        let extra = r#"{"schema": 1, "spine": "t/v1", "from": "kfgqpc", "to": "f", "note": "",
          "rows": [{"src": ["U+007A"], "dst": "U+005A", "kind": "cluster", "groups": ["g"]},
                   {"src": ["U+0031", "U+0032"], "dst": "U+0040", "kind": "sequence", "groups": ["n"]}]}"#;
        t.extend_from_json(extra).unwrap();
        assert_eq!(
            conv(&t, "a z 1 2")
                .unwrap()
                .chars
                .iter()
                .collect::<String>(),
            "aZ@"
        );
        let mut t = EncodingTable::from_json(TABLE).unwrap();
        assert!(
            t.extend_from_json(&extra.replace("U+007A", "U+0061"))
                .is_err()
        );
        assert!(
            t.extend_from_json(&extra.replace("\"t/v1\"", "\"t/v2\""))
                .is_err()
        );
    }

    #[test]
    fn the_identity_table_keeps_every_cluster_as_stored() {
        let t = EncodingTable::identity("t/v1");
        let c = conv(&t, "a zy q").unwrap();
        assert_eq!(c.chars.iter().collect::<String>(), "azyq");
        assert_eq!(c.spans[1].clusters, 1..2);
        assert_eq!(c.spans[1].chars, 1..3);
        assert!(c.spans.iter().all(|s| !s.lossy));
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
    #[test]
    fn hijazi_forms_replace_rows_and_carry_adjustments() {
        let mut t = EncodingTable::from_json(TABLE).unwrap();
        let forms = r#"{"schema": 1, "spine": "t/v1", "from": "kfgqpc", "to": "f", "note": "",
          "rows": [{"src": ["U+0061"], "dst": "U+0041 U+0042", "decision": "test",
            "adjust": [{"kind": "gap", "at": 0, "fu": 100},
                       {"kind": "shift", "at": 1, "cp": "U+0042", "dx": -5, "dy": 7}]}]}"#;
        t.extend_forms_from_json(forms).unwrap();
        let c = conv(&t, "b a").unwrap();
        assert_eq!(c.chars.iter().collect::<String>(), "BAB");
        assert_eq!(
            c.adjust,
            vec![
                Adjust::Gap { at: 1, fu: 100 },
                Adjust::Shift {
                    at: 2,
                    cp: Cps1('B'),
                    dx: -5,
                    dy: 7
                }
            ]
        );
        let outside = forms.replace("\"at\": 1, \"cp\"", "\"at\": 2, \"cp\"");
        assert!(
            EncodingTable::from_json(TABLE)
                .unwrap()
                .extend_forms_from_json(&outside)
                .is_err()
        );
    }
}

#[cfg(test)]
mod style_tests {
    use super::*;

    const ROWS: &str = r#"{"schema": 1, "note": "n", "rows": [
      {"from_style": "a", "to_style": "b", "from": "U+0301", "to": "U+0300", "evidence": "e"},
      {"from_style": "b", "to_style": "a", "from": "U+0300", "to": "U+0302", "evidence": "e"}]}"#;

    #[test]
    fn style_conversion_maps_codepoints_before_the_table() {
        let t = EncodingTable::identity("t/v1")
            .with_style_conversion(ROWS, "a", "b")
            .unwrap();
        let cl: Vec<&[char]> = vec![&['x', '\u{301}'], &['y']];
        assert_eq!(t.convert(&cl).unwrap().chars, ['x', '\u{300}', 'y']);
        assert_eq!(t.style_conversion().len(), 1);
        let none = EncodingTable::identity("t/v1")
            .with_style_conversion(ROWS, "a", "c")
            .unwrap();
        assert_eq!(none.convert(&cl).unwrap().chars, ['x', '\u{301}', 'y']);
    }

    #[test]
    fn a_codepoint_mapped_twice_is_an_error() {
        let twice = ROWS
            .replace(
                "\"from\": \"U+0300\", \"to\": \"U+0302\"",
                "\"from\": \"U+0301\", \"to\": \"U+0302\"",
            )
            .replace(
                "\"from_style\": \"b\", \"to_style\": \"a\"",
                "\"from_style\": \"a\", \"to_style\": \"b\"",
            );
        assert!(
            EncodingTable::identity("t/v1")
                .with_style_conversion(&twice, "a", "b")
                .is_err()
        );
    }
}
