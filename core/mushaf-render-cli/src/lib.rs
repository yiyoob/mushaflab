//! The reference renderer (T3.4, host only): loads a render profile and its data from the
//! repository and runs the core end to end: resolve, map lines, fit (shape + justify), draw.
//! SVG comes from `mushaf-draw`; PNG is rasterised here with resvg. All logic is in `core/`
//! crates (rule 7); this crate only reads files and calls them.

pub mod collide;

use std::fs;
use std::path::{Path, PathBuf};

use mushaf_draw::{DrawPage, Outlines, PageMeta, draw_page, to_svg};
use mushaf_justify::Regexps;
use mushaf_layout::fit::{
    FitData, FitPage, LineWidths, Profile, SurahNames, basmalah_words, fit_page,
};
use mushaf_layout::{Mapping, map_lines};
use mushaf_model::{DataSet, EditOverlay, Error};
use mushaf_resolve::{Mode, Resolved, Selection, resolve_profile};
use mushaf_shape::{EncodingTable, FontData};

/// Profiles by riwayah (Phase 3: Hafs only).
pub fn profile_path(riwayah: &str) -> Result<&'static str, Error> {
    match riwayah {
        "hafs" => Ok("data/render/hafs-uthmani-1441h.json"),
        other => Err(Error::new(format!(
            "riwayah {other:?} has no render profile yet (Phase 3 renders hafs; others are Phase 5)"
        ))),
    }
}

pub struct Renderer {
    pub repo: PathBuf,
    pub profile: Profile,
    pub data: DataSet,
    pub table: EncodingTable,
    pub widths: LineWidths,
    pub names: SurahNames,
    pub font: FontData,
    pub outlines: Outlines,
    pub basmalah: Vec<Vec<char>>,
    pub regexps: Regexps,
}

fn read(repo: &Path, rel: &str) -> Result<String, Error> {
    fs::read_to_string(repo.join(rel)).map_err(|e| Error::new(format!("{rel}: {e}")))
}

impl Renderer {
    pub fn load(repo: &Path, riwayah: &str) -> Result<Self, Error> {
        let profile = Profile::from_json(&read(repo, profile_path(riwayah)?)?)?;
        let data = mushaf_db::load_dir(&repo.join("data"))?;
        let table = EncodingTable::from_json(&read(repo, &profile.encoding)?)?;
        let spine = data
            .spines
            .get(&profile.spine)
            .ok_or_else(|| Error::new(format!("no spine {}", profile.spine)))?;
        if table.spine != spine.id {
            return Err(Error::new("encoding table is for another spine"));
        }
        let basmalah = basmalah_words(spine, &table)?;
        let bytes = fs::read(repo.join(&profile.font))
            .map_err(|e| Error::new(format!("{}: {e}", profile.font)))?;
        Ok(Renderer {
            repo: repo.to_path_buf(),
            widths: LineWidths::from_json(&read(repo, &profile.line_widths)?)?,
            names: SurahNames::from_json(&read(repo, &profile.surah_names)?)?,
            outlines: Outlines::new(&bytes)?,
            font: FontData::new(bytes)?,
            basmalah,
            table,
            data,
            profile,
            regexps: Regexps::new(),
        })
    }

    /// Resolves through the riwāyah's profile (T5.1). An overlay (free edits) resolves in free
    /// mode.
    pub fn resolve(&self, overlay: Option<&EditOverlay>) -> Result<Resolved, Error> {
        let sel = Selection {
            riwayah: self.profile.riwayah.clone(),
            ..Selection::default()
        };
        let mode = if overlay.is_some() {
            Mode::Free
        } else {
            Mode::Data
        };
        resolve_profile(&self.data, &sel, overlay, mode)
            .map_err(|e| Error::new(format!("resolve: {e:?}")))
    }

    pub fn map(&self, resolved: &Resolved) -> Result<Mapping, Error> {
        let spine = &self.data.spines[&self.profile.spine];
        let layout = self
            .data
            .layouts
            .get(&self.profile.layout_id)
            .ok_or_else(|| Error::new(format!("no layout {}", self.profile.layout_id)))?;
        map_lines(layout, spine, resolved).map_err(|e| Error::new(e.to_string()))
    }

    pub fn fit(&self, resolved: &Resolved, mapping: &Mapping, page: u32) -> Result<FitPage, Error> {
        fit_with(self, &self.font.shaper(), resolved, mapping, page)
    }

    fn fit_data(&self) -> FitData<'_> {
        FitData {
            profile: &self.profile,
            table: &self.table,
            widths: &self.widths,
            names: &self.names,
            basmalah: &self.basmalah,
            upem: i64::from(self.font.units_per_em()),
        }
    }

    pub fn draw(&self, fit: &FitPage) -> Result<DrawPage, Error> {
        let meta = PageMeta {
            layout_id: &self.profile.layout_id,
            script_id: &self.profile.script_id,
            riwayah: &self.profile.riwayah,
            font: &self.profile.font,
        };
        draw_page(fit, &self.profile.geometry, &self.outlines, &meta)
    }

    /// Resolve, fit and draw one page.
    pub fn render(&self, page: u32, overlay: Option<&EditOverlay>) -> Result<DrawPage, Error> {
        let resolved = self.resolve(overlay)?;
        let mapping = self.map(&resolved)?;
        self.draw(&self.fit(&resolved, &mapping, page)?)
    }
}

/// Fit a page with a given shaper (whose word cache may be shared across pages, T3.6).
pub fn fit_with(
    r: &Renderer,
    shaper: &mushaf_shape::Shaper,
    resolved: &Resolved,
    mapping: &Mapping,
    page: u32,
) -> Result<FitPage, Error> {
    fit_page(shaper, &r.regexps, &r.fit_data(), resolved, mapping, page)
}

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
