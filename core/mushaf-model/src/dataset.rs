//! T2.2: everything under a data root, loaded either from the tracked files or from the rows of
//! `mushaf.db` (schema v4). Both paths build the same `DataSet`, and the parity tests in
//! `mushaf-db` check that they are equal. Pure: callers supply file contents or rows.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::{
    AyahMap, Cluster, Cps, Error, Item, ItemKind, Layout, Line, OptionGroup, PatchFile, Register,
    RiwayahProfile, RuleFile, Spine, from_json, from_yaml,
};

/// File contents by path relative to the data root, with `/` separators.
pub type DataFiles = BTreeMap<String, String>;

/// The top-level directories of a data root that hold loadable data.
pub const DATA_DIRS: [&str; 7] = [
    "spine",
    "layouts",
    "patches",
    "options",
    "ayah_maps",
    "profiles",
    "rules",
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataSet {
    /// By spine id (`hafs/v1`).
    pub spines: BTreeMap<String, Spine>,
    /// By layout id.
    pub layouts: BTreeMap<String, Layout>,
    /// By path relative to the data root (`patches/raw/warsh.yaml`).
    pub patch_files: BTreeMap<String, PatchFile>,
    /// By group id, from all option files.
    pub option_groups: BTreeMap<String, OptionGroup>,
    /// By riwayah id.
    pub ayah_maps: BTreeMap<String, AyahMap>,
    /// Riwāyah profiles (T5.1), by riwayah id.
    pub profiles: BTreeMap<String, RiwayahProfile>,
    /// Rule files (T6.1), by path relative to the data root (`rules/hafs.yaml`).
    pub rule_files: BTreeMap<String, RuleFile>,
    /// The rule register (`rules/register.yaml`); it must agree with the rule files.
    pub register: Register,
}

/// What a file under the data root is, or `None` if the loader does not read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    SpineItems(String),
    SpineManifest(String),
    Layout,
    PatchFile,
    OptionFile,
    AyahMap,
    Profile,
    RuleFile,
    RuleRegister,
}

/// The hand-written register's path under the data root; generated rule files have their
/// own `rules/<name>.register.yaml`.
pub const REGISTER_PATH: &str = "rules/register.yaml";

/// Classifies a root-relative path. `.gitkeep` and layout correction inputs (read by the
/// pipeline, not the core) are not inputs; any other unexpected file is an error.
pub fn classify(path: &str) -> Result<Option<Input>, Error> {
    let parts: Vec<&str> = path.split('/').collect();
    let name = parts.last().copied().unwrap_or_default();
    if name == ".gitkeep" {
        return Ok(None);
    }
    let input = match parts.as_slice() {
        ["spine", n, v, "items.jsonl"] => Some(Input::SpineItems(format!("{n}/{v}"))),
        ["spine", n, v, "manifest.json"] => Some(Input::SpineManifest(format!("{n}/{v}"))),
        ["layouts", f] if f.ends_with(".corrections.json") => return Ok(None),
        ["layouts", f] if f.ends_with(".json") => Some(Input::Layout),
        ["patches", .., f] if f.ends_with(".yaml") => Some(Input::PatchFile),
        ["options", f] if f.ends_with(".yaml") => Some(Input::OptionFile),
        ["ayah_maps", f] if f.ends_with(".yaml") => Some(Input::AyahMap),
        ["profiles", f] if f.ends_with(".yaml") => Some(Input::Profile),
        ["rules", "README.md"] => return Ok(None),
        ["rules", f] if *f == "register.yaml" || f.ends_with(".register.yaml") => {
            Some(Input::RuleRegister)
        }
        ["rules", f] if f.ends_with(".yaml") => Some(Input::RuleFile),
        _ => None,
    };
    input
        .map(Some)
        .ok_or_else(|| Error::new(format!("unexpected file in data root: {path}")))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpineSourceInfo {
    #[serde(rename = "id")]
    _id: String,
    #[serde(rename = "member")]
    _member: String,
    #[serde(rename = "sha256")]
    _sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Counts {
    items: BTreeMap<String, u64>,
    clusters: BTreeMap<String, u64>,
}

/// `spine/<name>/v<k>/manifest.json` (D11).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpineManifest {
    script_id: String,
    spine_version: u32,
    #[serde(rename = "source")]
    _source: SpineSourceInfo,
    #[serde(rename = "codepoint_table_sha256")]
    _codepoint_table_sha256: String,
    #[serde(rename = "unicode_version")]
    _unicode_version: String,
    counts: Counts,
}

fn kind_name<T: serde::Serialize>(k: &T) -> String {
    serde_json::to_value(k)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn spine_from_files(id: &str, manifest: &str, items: &str) -> Result<Spine, Error> {
    let err = |e: Error| Error::new(format!("spine {id}: {e}"));
    let m: SpineManifest = from_json(manifest).map_err(err)?;
    let items: Vec<Item> = items
        .lines()
        .enumerate()
        .map(|(n, l)| {
            from_json(l).map_err(|e| Error::new(format!("spine {id} line {}: {e}", n + 1)))
        })
        .collect::<Result<_, _>>()?;
    let mut items_n: BTreeMap<String, u64> = BTreeMap::new();
    let mut clusters_n: BTreeMap<String, u64> = BTreeMap::new();
    for it in &items {
        *items_n.entry(kind_name(&it.kind)).or_default() += 1;
        for c in &it.clusters {
            *clusters_n.entry(kind_name(&c.kind)).or_default() += 1;
        }
    }
    if items_n != m.counts.items || clusters_n != m.counts.clusters {
        return Err(Error::new(format!(
            "spine {id}: item or cluster counts differ from manifest"
        )));
    }
    Spine::new(id.into(), m.script_id, m.spine_version, items)
}

fn ascii(path: &str, text: &str) -> Result<(), Error> {
    if text.is_ascii() {
        return Ok(());
    }
    Err(Error::new(format!(
        "{path}: must be ASCII (write codepoints as U+XXXX)"
    )))
}

fn insert_unique<V>(
    map: &mut BTreeMap<String, V>,
    key: String,
    v: V,
    what: &str,
) -> Result<(), Error> {
    if map.contains_key(&key) {
        return Err(Error::new(format!("{what} {key:?} is defined twice")));
    }
    map.insert(key, v);
    Ok(())
}

impl DataSet {
    pub fn from_files(files: &DataFiles) -> Result<DataSet, Error> {
        let mut ds = DataSet::default();
        let mut spine_parts: BTreeMap<String, (Option<&str>, Option<&str>)> = BTreeMap::new();
        for (path, text) in files {
            let at = |e: Error| Error::new(format!("{path}: {e}"));
            match classify(path)? {
                None => {}
                Some(Input::SpineItems(id)) => spine_parts.entry(id).or_default().1 = Some(text),
                Some(Input::SpineManifest(id)) => spine_parts.entry(id).or_default().0 = Some(text),
                Some(Input::Layout) => {
                    let l: Layout = from_json(text).map_err(at)?;
                    if !path.ends_with(&format!("/{}.json", l.layout_id)) {
                        return Err(Error::new(format!(
                            "{path}: file name is not its layout_id"
                        )));
                    }
                    insert_unique(&mut ds.layouts, l.layout_id.clone(), l, "layout")?;
                }
                Some(Input::PatchFile) => {
                    ascii(path, text)?;
                    ds.patch_files
                        .insert(path.clone(), from_yaml(text).map_err(at)?);
                }
                Some(Input::OptionFile) => {
                    for g in from_yaml::<Vec<OptionGroup>>(text).map_err(at)? {
                        insert_unique(&mut ds.option_groups, g.id.clone(), g, "option group")?;
                    }
                }
                Some(Input::AyahMap) => {
                    ascii(path, text)?;
                    let m: AyahMap = from_yaml(text).map_err(at)?;
                    insert_unique(&mut ds.ayah_maps, m.riwayah.clone(), m, "ayah map")?;
                }
                Some(Input::Profile) => {
                    ascii(path, text)?;
                    let p: RiwayahProfile = from_yaml(text).map_err(at)?;
                    if *path != format!("profiles/{}.yaml", p.riwayah) {
                        return Err(Error::new(format!("{path}: file name is not its riwayah")));
                    }
                    insert_unique(&mut ds.profiles, p.riwayah.clone(), p, "profile")?;
                }
                Some(Input::RuleFile) => {
                    ascii(path, text)?;
                    ds.rule_files
                        .insert(path.clone(), from_yaml(text).map_err(at)?);
                }
                Some(Input::RuleRegister) => {
                    // Several register files (hand-written and generated) form one register.
                    let r: Register = from_yaml(text).map_err(at)?;
                    ds.register.rules.extend(r.rules);
                }
            }
        }
        for (id, parts) in spine_parts {
            let (Some(manifest), Some(items)) = parts else {
                return Err(Error::new(format!(
                    "spine {id}: needs items.jsonl and manifest.json"
                )));
            };
            ds.spines
                .insert(id.clone(), spine_from_files(&id, manifest, items)?);
        }
        ds.validate()?;
        Ok(ds)
    }

    pub fn from_rows(rows: DbRows) -> Result<DataSet, Error> {
        let mut ds = DataSet::default();
        let mut clusters: BTreeMap<(String, String), Vec<(i64, Cluster)>> = BTreeMap::new();
        for c in rows.clusters {
            let cluster = Cluster {
                id: c.id.parse()?,
                kind: enum_from(&c.kind)?,
                cps: Cps::from_text(&c.text),
            };
            clusters
                .entry((c.spine_id, c.item_id))
                .or_default()
                .push((c.idx, cluster));
        }
        let mut items: BTreeMap<String, Vec<(i64, Item)>> = BTreeMap::new();
        for r in rows.items {
            let mut cs = clusters
                .remove(&(r.spine_id.clone(), r.id.clone()))
                .unwrap_or_default();
            cs.sort_by_key(|(idx, _)| *idx);
            let item = Item {
                id: r.id.parse()?,
                kind: enum_from::<ItemKind>(&r.kind)?,
                sep_before: Cps::from_text(&r.sep_before),
                sep_after: Cps::from_text(&r.sep_after),
                clusters: cs.into_iter().map(|(_, c)| c).collect(),
            };
            items.entry(r.spine_id).or_default().push((r.ord, item));
        }
        if let Some((spine, item)) = clusters.keys().next() {
            return Err(Error::new(format!(
                "DB: clusters for unknown item {spine} {item}"
            )));
        }
        for s in rows.spines {
            let mut its = items.remove(&s.spine_id).unwrap_or_default();
            its.sort_by_key(|(ord, _)| *ord);
            let version =
                u32::try_from(s.version).map_err(|_| Error::new("DB: bad spine version"))?;
            let spine = Spine::new(
                s.spine_id.clone(),
                s.script_id,
                version,
                its.into_iter().map(|(_, i)| i).collect(),
            )?;
            ds.spines.insert(s.spine_id, spine);
        }
        if let Some(id) = items.keys().next() {
            return Err(Error::new(format!("DB: items for unknown spine {id}")));
        }
        let mut lines: BTreeMap<String, Vec<Line>> = BTreeMap::new();
        for l in rows.lines {
            let line = Line::from_columns(
                l.page,
                l.line,
                &l.kind,
                l.surah,
                l.first.as_deref(),
                l.last.as_deref(),
                l.centered,
                l.correction,
                l.fit.as_deref(),
            )?;
            lines.entry(l.layout_id).or_default().push(line);
        }
        for l in rows.layouts {
            let mut ls = lines.remove(&l.layout_id).unwrap_or_default();
            ls.sort_by_key(|x| (x.page, x.line));
            let layout = Layout {
                layout_id: l.layout_id.clone(),
                script_id: l.script_id,
                spine: l.spine_id,
                riwayah: l.riwayah,
                pages: l.pages,
                lines_per_page: l.lines_per_page,
                sources: from_json(&l.sources)?,
                lines: ls,
            };
            ds.layouts.insert(l.layout_id, layout);
        }
        if let Some(id) = lines.keys().next() {
            return Err(Error::new(format!("DB: lines for unknown layout {id}")));
        }
        for b in rows.patch_files {
            let f: PatchFile =
                from_json(&b.body).map_err(|e| Error::new(format!("DB {}: {e}", b.key)))?;
            if f.spine != b.spine_id {
                return Err(Error::new(format!(
                    "DB {}: spine column disagrees with body",
                    b.key
                )));
            }
            ds.patch_files.insert(b.key, f);
        }
        for b in rows.option_groups {
            let g: OptionGroup = from_json(&b.body)?;
            if g.id != b.key {
                return Err(Error::new(format!(
                    "DB option group {}: id disagrees with body",
                    b.key
                )));
            }
            ds.option_groups.insert(b.key, g);
        }
        for b in rows.ayah_maps {
            let m: AyahMap = from_json(&b.body)?;
            if m.riwayah != b.key || m.spine != b.spine_id {
                return Err(Error::new(format!(
                    "DB ayah map {}: columns disagree with body",
                    b.key
                )));
            }
            ds.ayah_maps.insert(b.key, m);
        }
        for b in rows.profiles {
            let p: RiwayahProfile = from_json(&b.body)?;
            if p.riwayah != b.key || p.spine != b.spine_id {
                return Err(Error::new(format!(
                    "DB profile {}: columns disagree with body",
                    b.key
                )));
            }
            ds.profiles.insert(b.key, p);
        }
        for b in rows.rule_files {
            let f: RuleFile =
                from_json(&b.body).map_err(|e| Error::new(format!("DB {}: {e}", b.key)))?;
            if f.spine != b.spine_id {
                return Err(Error::new(format!(
                    "DB {}: spine column disagrees with body",
                    b.key
                )));
            }
            ds.rule_files.insert(b.key, f);
        }
        for b in rows.rule_register {
            if classify(&b.key)? != Some(Input::RuleRegister) {
                return Err(Error::new(format!("DB: unexpected register row {}", b.key)));
            }
            let r: Register = from_json(&b.body)?;
            ds.register.rules.extend(r.rules);
        }
        ds.validate()?;
        Ok(ds)
    }

    /// Cross-references: every layout, patch file, ayah map and profile names a loaded spine,
    /// every layout is ordered, and a profile's patch files and ayah map exist on its spine.
    fn validate(&self) -> Result<(), Error> {
        let known = |s: &str, what: &str| {
            if self.spines.contains_key(s) {
                Ok(())
            } else {
                Err(Error::new(format!("{what} refers to unknown spine {s:?}")))
            }
        };
        for l in self.layouts.values() {
            known(&l.spine, &format!("layout {}", l.layout_id))?;
            l.validate()?;
        }
        for (path, f) in &self.patch_files {
            known(&f.spine, path)?;
        }
        for m in self.ayah_maps.values() {
            known(&m.spine, &format!("ayah map {}", m.riwayah))?;
        }
        for (path, f) in &self.rule_files {
            known(&f.spine, path)?;
        }
        let problems = self
            .register
            .check(self.rule_files.iter().map(|(p, f)| (p.as_str(), f)));
        if !problems.is_empty() {
            return Err(Error::new(format!(
                "rules and register disagree: {}",
                problems.join("; ")
            )));
        }
        for p in self.profiles.values() {
            let what = format!("profile {}", p.riwayah);
            known(&p.spine, &what)?;
            for path in &p.patch_stack {
                match self.patch_files.get(path) {
                    Some(f) if f.spine == p.spine => {}
                    Some(_) => {
                        return Err(Error::new(format!("{what}: {path} is on another spine")));
                    }
                    None => return Err(Error::new(format!("{what}: no patch file {path}"))),
                }
            }
            for path in &p.rule_stack {
                match self.rule_files.get(path) {
                    Some(f) if f.spine == p.spine => {}
                    Some(_) => {
                        return Err(Error::new(format!("{what}: {path} is on another spine")));
                    }
                    None => return Err(Error::new(format!("{what}: no rule file {path}"))),
                }
            }
            if let Some(m) = &p.ayah_map {
                match self.ayah_maps.get(m) {
                    Some(a) if a.spine == p.spine && a.riwayah == p.riwayah => {}
                    _ => return Err(Error::new(format!("{what}: no ayah map {m} for it"))),
                }
            }
        }
        Ok(())
    }
}

fn enum_from<T: DeserializeOwned>(s: &str) -> Result<T, Error> {
    serde_json::from_value(serde_json::Value::String(s.into()))
        .map_err(|e| Error::new(format!("{s:?}: {e}")))
}

/// Rows of `mushaf.db` (schema v4), as read by `mushaf-db`. Text columns hold real codepoints.
#[derive(Clone, Debug, Default)]
pub struct DbRows {
    pub spines: Vec<SpineRow>,
    pub items: Vec<ItemRow>,
    pub clusters: Vec<ClusterRow>,
    pub layouts: Vec<LayoutRow>,
    pub lines: Vec<LineRow>,
    pub patch_files: Vec<BodyRow>,
    pub option_groups: Vec<BodyRow>,
    pub ayah_maps: Vec<BodyRow>,
    pub profiles: Vec<BodyRow>,
    pub rule_files: Vec<BodyRow>,
    /// One row per register file (`rules/register.yaml`, `rules/<name>.register.yaml`).
    pub rule_register: Vec<BodyRow>,
}

#[derive(Clone, Debug)]
pub struct SpineRow {
    pub spine_id: String,
    pub script_id: String,
    pub version: i64,
}

#[derive(Clone, Debug)]
pub struct ItemRow {
    pub spine_id: String,
    pub ord: i64,
    pub id: String,
    pub kind: String,
    pub sep_before: String,
    pub sep_after: String,
}

#[derive(Clone, Debug)]
pub struct ClusterRow {
    pub spine_id: String,
    pub item_id: String,
    pub idx: i64,
    pub id: String,
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct LayoutRow {
    pub layout_id: String,
    pub script_id: String,
    pub spine_id: String,
    pub pages: u32,
    pub lines_per_page: u32,
    pub sources: String,
    pub riwayah: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LineRow {
    pub layout_id: String,
    pub page: u32,
    pub line: u32,
    pub kind: String,
    pub surah: Option<u32>,
    pub first: Option<String>,
    pub last: Option<String>,
    pub centered: bool,
    pub correction: Option<String>,
    pub fit: Option<String>,
}

/// A row holding canonical JSON: `key` is the path (patch files), group id or riwayah id.
#[derive(Clone, Debug)]
pub struct BodyRow {
    pub key: String,
    pub spine_id: String,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! fixture {
        ($p:literal) => {
            (
                $p.to_string(),
                include_str!(concat!("../../../tests/fixtures/synthetic/", $p)).to_string(),
            )
        };
    }

    fn synthetic() -> DataFiles {
        [
            fixture!("spine/test/v1/items.jsonl"),
            fixture!("spine/test/v1/manifest.json"),
            fixture!("layouts/test-layout.json"),
            fixture!("patches/raw/r1.yaml"),
            fixture!("patches/curated/q1.yaml"),
            fixture!("options/test.yaml"),
            fixture!("ayah_maps/r1.yaml"),
            fixture!("profiles/r1.yaml"),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn loads_the_synthetic_root() {
        let ds = DataSet::from_files(&synthetic()).unwrap();
        assert_eq!(ds.spines["test/v1"].items.len(), 10);
        assert_eq!(ds.layouts["test-layout"].lines.len(), 6);
        assert_eq!(ds.patch_files["patches/raw/r1.yaml"].patches.len(), 10);
        assert_eq!(
            ds.option_groups.keys().collect::<Vec<_>>(),
            ["r3.tariq", "test.other", "test.toggle"]
        );
        assert_eq!(ds.ayah_maps["r1"].entries.len(), 4);
        assert_eq!(ds.profiles["r1"].patch_stack.len(), 2);
    }

    #[test]
    fn rejects_bad_roots() {
        type Change = (&'static str, fn(&mut DataFiles));
        let cases: [Change; 9] = [
            ("profile names a missing patch file", |f| {
                let p = f.get_mut("profiles/r1.yaml").unwrap();
                *p = p.replace("patches/raw/r1.yaml", "patches/raw/r2.yaml");
            }),
            ("profile names a missing ayah map", |f| {
                let p = f.get_mut("profiles/r1.yaml").unwrap();
                *p = p.replace("ayah_map: \"r1\"", "ayah_map: \"r2\"");
            }),
            ("profile file name differs from its riwayah", |f| {
                let p = f.remove("profiles/r1.yaml").unwrap();
                f.insert("profiles/r2.yaml".into(), p);
            }),
            ("unexpected file", |f| {
                f.insert("patches/raw/notes.txt".into(), String::new());
            }),
            ("non-ASCII patch file", |f| {
                let p = f.get_mut("patches/raw/r1.yaml").unwrap();
                *p = p.replace("synthetic", "synth\u{e9}tic");
            }),
            ("count mismatch", |f| {
                let p = f.get_mut("spine/test/v1/manifest.json").unwrap();
                *p = p.replace("\"word\": 6", "\"word\": 7");
            }),
            ("missing manifest", |f| {
                f.remove("spine/test/v1/manifest.json");
            }),
            ("duplicate option group", |f| {
                let g = f["options/test.yaml"].clone();
                f.insert("options/again.yaml".into(), g);
            }),
            ("unknown spine", |f| {
                let p = f.get_mut("patches/raw/r1.yaml").unwrap();
                *p = p.replace("spine: \"test/v1\"", "spine: \"test/v2\"");
            }),
        ];
        for (what, change) in cases {
            let mut files = synthetic();
            change(&mut files);
            assert!(DataSet::from_files(&files).is_err(), "accepted: {what}");
        }
        let mut files = synthetic();
        files.insert("layouts/.gitkeep".into(), String::new());
        files.insert(
            "layouts/test-layout.corrections.json".into(),
            "not read".into(),
        );
        assert!(DataSet::from_files(&files).is_ok());
    }
}
