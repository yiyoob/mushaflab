//! The reference renderer (T3.4, host only): the page engine (`mushaf-engine`, D56) reading
//! the repository from disk, plus SVG and PNG output. PNG is rasterised here with resvg. All
//! logic is in `core/` crates (rule 7); this crate only reads files and calls them.

pub mod collide;

pub use mushaf_engine::*;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mushaf_draw::{DrawPage, to_svg};
use mushaf_model::{Error, RiwayahProfile};

/// Rasterise an SVG page to PNG at `width_px` (height keeps the page's proportions).
pub fn png(svg: &str, width_px: u32) -> Result<Vec<u8>, Error> {
    let opts = resvg::usvg::Options::default();
    let tree =
        resvg::usvg::Tree::from_str(svg, &opts).map_err(|e| Error::new(format!("svg: {e}")))?;
    let size = tree.size();
    let scale = width_px as f32 / size.width();
    let height_px = (size.height() * scale).round() as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width_px, height_px)
        .ok_or_else(|| Error::new("png: empty page"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| Error::new(format!("png: {e}")))
}

/// The page as SVG (`mushaf-draw`'s backend).
pub fn svg(page: &DrawPage) -> String {
    to_svg(page)
}

fn strings<'a>(v: &'a serde_json::Value, out: &mut BTreeSet<&'a str>) {
    match v {
        serde_json::Value::String(s) => {
            out.insert(s);
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

/// The files of the web bundle (D57), by repository-relative path, byte for byte:
/// - the data root's data files, except patch files no riwāyah profile stacks (the raw
///   patches are the round-trip's input, D22; the engine never resolves them);
/// - `data/render/*`, `data/encodings/*` and `data/approvals/*` (read-only, for the
///   provenance screen);
/// - every font a render profile or the style audit names.
pub fn bundle_files(repo: &Path) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    let read =
        |rel: &str| std::fs::read(repo.join(rel)).map_err(|e| Error::new(format!("{rel}: {e}")));
    let data = mushaf_model::read_tree(&repo.join("data"))?;
    let mut stacked = BTreeSet::new();
    for (k, v) in data.iter().filter(|(k, _)| k.starts_with("profiles/")) {
        let p: RiwayahProfile =
            mushaf_model::from_yaml(v).map_err(|e| Error::new(format!("{k}: {e}")))?;
        stacked.extend(p.patch_stack);
    }
    let mut out = BTreeMap::new();
    for (k, v) in data {
        if k.starts_with("patches/") && !stacked.contains(&k) {
            continue;
        }
        out.insert(format!("data/{k}"), v.into_bytes());
    }
    let mut named = BTreeSet::new();
    for dir in ["data/render", "data/encodings", "data/approvals"] {
        let mut names: Vec<_> = std::fs::read_dir(repo.join(dir))
            .map_err(|e| Error::new(format!("{dir}: {e}")))?
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
            .filter(|n| !n.starts_with('.'))
            .collect();
        names.sort();
        for n in names {
            let rel = format!("{dir}/{n}");
            let bytes = read(&rel)?;
            if n.ends_with(".json") && dir == "data/render" {
                let v: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|e| Error::new(format!("{rel}: {e}")))?;
                let mut s = BTreeSet::new();
                strings(&v, &mut s);
                named.extend(
                    s.into_iter()
                        .filter(|s| s.starts_with("fonts/"))
                        .map(String::from),
                );
            }
            out.insert(rel, bytes);
        }
    }
    for f in named {
        out.insert(f.clone(), read(&f)?);
    }
    Ok(out)
}
