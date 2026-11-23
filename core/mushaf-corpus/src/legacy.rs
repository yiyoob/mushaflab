//! The corpus as the engine's current input types (M1.4, D73): the Ḥafṣ word table as the spine
//! `hafs/v1`, and each corpus layout as a `mushaf_model::Layout`. A bridge until the engine reads
//! the corpus model directly; the legacy riwāyāt's patches keep targeting `hafs/v1` until M3.
//!
//! `corpus/text/words.tsv` was generated from the spine `hafs/v1` (M0.4), so the spine is named
//! after it. Clusters come from `mushaf_text::clusterize`, the same rules the legacy clusterizer
//! used; the equality with the legacy spine and layout is tested on the real data.

use std::collections::BTreeMap;

use mushaf_model::{Cluster, ClusterKind, Cps, Error, Item, ItemKind, Layout, Line, Spine};

use crate::{Corpus, LayoutLine};

/// The spine the word table replaces, and its script and version (data/spine/hafs/v1).
pub const SPINE_ID: &str = "hafs/v1";
pub const SPINE_SCRIPT: &str = "uthmani";
pub const SPINE_VERSION: u32 = 1;

fn cluster_kind(k: mushaf_text::ClusterKind) -> ClusterKind {
    match k {
        mushaf_text::ClusterKind::Letter => ClusterKind::Letter,
        mushaf_text::ClusterKind::Sign => ClusterKind::Sign,
        mushaf_text::ClusterKind::AyahNumber => ClusterKind::AyahNumber,
        mushaf_text::ClusterKind::Digit => ClusterKind::Digit,
        mushaf_text::ClusterKind::Control => ClusterKind::Control,
    }
}

fn cps(cell: &str, at: &str) -> Result<Cps, Error> {
    if cell.is_empty() {
        return Ok(Cps::default());
    }
    cell.parse()
        .map_err(|e: Error| Error::new(format!("{at}: separator {cell:?}: {e}")))
}

/// The word table as the spine `hafs/v1`.
pub fn spine(c: &Corpus) -> Result<Spine, Error> {
    let mut items = Vec::with_capacity(c.words.len());
    for w in &c.words {
        let at = w.id.to_string();
        let id = mushaf_model::ItemRef::spine(
            u32::from(w.id.surah),
            u32::from(w.id.ayah),
            u32::from(w.id.word),
        );
        let kind = match w.kind.as_str() {
            "word" => ItemKind::Word,
            "ayah_end" => ItemKind::AyahEnd,
            "hizb" => ItemKind::Hizb,
            other => return Err(Error::new(format!("{at}: unknown kind {other:?}"))),
        };
        let clusters = mushaf_text::clusterize(&w.text)
            .map_err(|e| Error::new(format!("{at}: {e}")))?
            .into_iter()
            .enumerate()
            .map(|(k, cl)| Cluster {
                id: id.cluster(k as u32 + 1),
                kind: cluster_kind(cl.kind),
                cps: Cps::from_text(&cl.text),
            })
            .collect();
        items.push(Item {
            sep_before: cps(&w.sep_before, &at)?,
            sep_after: cps(&w.sep_after, &at)?,
            id,
            kind,
            clusters,
        });
    }
    Spine::new(SPINE_ID.into(), SPINE_SCRIPT.into(), SPINE_VERSION, items)
}

/// Every corpus layout as a `Layout` on the spine `hafs/v1`, by layout id. A surah header line
/// takes the surah of the text line that follows it on its page (checked by `Corpus::check`);
/// a corrected line carries its correction's evidence.
pub fn layouts(c: &Corpus) -> Result<BTreeMap<String, Layout>, Error> {
    let mut by_dir: BTreeMap<&str, Vec<&LayoutLine>> = BTreeMap::new();
    for l in &c.layout_lines {
        let dir = l.file.strip_suffix("lines.tsv").unwrap_or(&l.file);
        by_dir.entry(dir).or_default().push(l);
    }
    let mut out = BTreeMap::new();
    for (path, lay) in &c.layouts {
        let dir = path.strip_suffix("layout.yaml").unwrap_or(path);
        let rows = by_dir.get(dir).map(Vec::as_slice).unwrap_or_default();
        let mut lines = Vec::with_capacity(rows.len());
        for (k, l) in rows.iter().enumerate() {
            let surah = if l.kind == "surah_name" {
                let next = rows[k + 1..]
                    .iter()
                    .take_while(|m| m.page == l.page)
                    .find_map(|m| m.first)
                    .ok_or_else(|| {
                        Error::new(format!(
                            "{}:{}: surah_name not followed by a text line on its page",
                            l.file, l.line
                        ))
                    })?;
                Some(u32::from(next.surah))
            } else {
                None
            };
            let correction = lay
                .corrections
                .iter()
                .find(|x| x.page == l.page && x.line == l.line_no)
                .map(|x| x.evidence.clone());
            let first = l.first.map(|w| w.to_string());
            let last = l.last.map(|w| w.to_string());
            lines.push(Line::from_columns(
                u32::from(l.page),
                u32::from(l.line_no),
                &l.kind,
                surah,
                first.as_deref(),
                last.as_deref(),
                l.centered,
                correction,
                None,
            )?);
        }
        let sources = lay
            .sources
            .iter()
            .flat_map(|s| s.pins.iter().map(|(k, v)| (k.clone(), v.clone())))
            .collect();
        let layout = Layout {
            layout_id: lay.layout_id.clone(),
            script_id: lay.script_id.clone(),
            spine: SPINE_ID.into(),
            riwayah: None,
            pages: u32::from(lay.pages),
            lines_per_page: u32::from(lay.lines_per_page),
            sources,
            lines,
        };
        layout
            .validate()
            .map_err(|e| Error::new(format!("{path}: {e}")))?;
        out.insert(lay.layout_id.clone(), layout);
    }
    Ok(out)
}

/// Put the corpus's spine and layouts into a data set, replacing the legacy ones of the same
/// ids, and validate the result (patch targets, layout references).
pub fn apply(c: &Corpus, ds: &mut mushaf_model::DataSet) -> Result<(), Error> {
    if c.words.is_empty() {
        return Ok(());
    }
    ds.spines.insert(SPINE_ID.into(), spine(c)?);
    ds.layouts.extend(layouts(c)?);
    ds.validate()
}
