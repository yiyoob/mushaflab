//! The reference renderer (T3.4, host only): loads a render profile and its data from the
//! repository and runs the core end to end: resolve, map lines, fit (shape + justify), draw.
//! SVG comes from `mushaf-draw`; PNG is rasterised here with resvg. All logic is in `core/`
//! crates (rule 7); this crate only reads files and calls them.

pub mod collide;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mushaf_draw::{DrawPage, Outlines, PageMeta, draw_page, to_svg};
use mushaf_justify::Regexps;
use mushaf_layout::fit::{
    FitData, FitPage, LineWidths, Profile, SurahNames, basmalah_words, fit_page,
};
use mushaf_layout::{Mapping, map_lines, map_lines_on};
use mushaf_model::{DataSet, EditOverlay, Error};
use mushaf_resolve::{Mode, Resolved, Selection, resolve_profile};
use mushaf_shape::{EncodingTable, Exceptions, FontData, Violation, check_word};

/// A riwāyah's default display style (D36): Maghribi for Warsh and Qālūn, Ḥijāzī for the rest.
pub fn default_style(riwayah: &str) -> Result<&'static str, Error> {
    match riwayah {
        "warsh" | "qalun" => Ok("maghribi"),
        "hafs" | "shuba" | "bazzi" | "qunbul" | "duri_abuamr" | "susi" => Ok("hijazi"),
        other => Err(Error::new(format!("unknown riwayah {other:?}"))),
    }
}

/// Render profiles by (riwāyah, style) (D36): Ḥafṣ on the 1441H layout, the others on their
/// native KFGQPC layouts (T5.2); a non-default style has `data/render/<riwāyah>-<style>.json`
/// (T6.10, D44), used only when the T6.9 audit marks the combination available.
pub fn profile_path(riwayah: &str, style: &str) -> Result<String, Error> {
    if !matches!(style, "hijazi" | "maghribi") {
        return Err(Error::new(format!("unknown style {style:?}")));
    }
    if style != default_style(riwayah)? {
        return Ok(format!("data/render/{riwayah}-{style}.json"));
    }
    Ok(match riwayah {
        "hafs" => "data/render/hafs-uthmani-1441h.json".into(),
        _ => format!("data/render/{riwayah}-uthmani-kfgqpc.json"),
    })
}

fn hijazi_from_maghribi(p: &Profile) -> bool {
    p.style == "hijazi" && p.text_style.as_deref() == Some("maghribi")
}

/// The T6.9 audit of every (riwāyah, non-default style) (D44).
pub const STYLE_AVAILABILITY: &str = "data/render/style-availability.json";

/// Refuses a non-default style the T6.9 audit does not mark available, with the audit's counts.
pub fn check_style_available(repo: &Path, riwayah: &str, style: &str) -> Result<(), Error> {
    if style == default_style(riwayah)? {
        return Ok(());
    }
    let doc: serde_json::Value = serde_json::from_str(&read(repo, STYLE_AVAILABILITY)?)
        .map_err(|e| Error::new(format!("{STYLE_AVAILABILITY}: {e}")))?;
    let entry = doc["entries"]
        .as_array()
        .and_then(|es| {
            es.iter()
                .find(|e| e["riwayah"] == riwayah && e["style"] == style)
        })
        .ok_or_else(|| {
            Error::new(format!(
                "{STYLE_AVAILABILITY}: no entry for {riwayah} {style}"
            ))
        })?;
    if entry["available"] == true {
        return Ok(());
    }
    Err(Error::new(format!(
        "style {style:?} of riwayah {riwayah:?} is not available (D44, T6.9 audit in \
         {STYLE_AVAILABILITY}): {} of {} words cannot convert, {} fail rule 6",
        entry["unconvertible_words"], entry["words"], entry["rule6_failing_words"]
    )))
}

/// Owner-approved per-glyph rule-6 exceptions (D27, G-100).
pub const EXCEPTIONS: &str = "data/render/kfgqpc-rule6-exceptions.json";

/// A page checked against rule 6 (T5.2): drawn, or blocked because a word cannot be converted
/// to the font's encoding yet (G-102). `violations` lists every word that breaks rule 6 apart
/// from the owner's exceptions; a page with any is not shown as fit (the caller decides).
pub struct CheckedPage {
    pub fit: Option<FitPage>,
    pub blocked: Option<String>,
    pub violations: Vec<(String, Violation)>,
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
    /// The riwāyah's resolved text without edits: what a native layout's anchors name.
    pub base: Resolved,
    /// Owner-approved per-glyph rule-6 exceptions (D27).
    pub exceptions: Exceptions,
    pub basmalah: Vec<Vec<char>>,
    pub regexps: Regexps,
}

fn read(repo: &Path, rel: &str) -> Result<String, Error> {
    fs::read_to_string(repo.join(rel)).map_err(|e| Error::new(format!("{rel}: {e}")))
}

impl Renderer {
    /// The riwāyah in its default style (D36).
    pub fn load(repo: &Path, riwayah: &str) -> Result<Self, Error> {
        Self::load_style(repo, riwayah, default_style(riwayah)?)
    }

    pub fn load_style(repo: &Path, riwayah: &str, style: &str) -> Result<Self, Error> {
        check_style_available(repo, riwayah, style)?;
        let profile = Profile::from_json(&read(repo, &profile_path(riwayah, style)?)?)?;
        if profile.riwayah != riwayah || profile.style != style {
            return Err(Error::new(format!(
                "render profile {} is for {} in style {}",
                profile.profile, profile.riwayah, profile.style
            )));
        }
        let data = mushaf_db::load_dir(&repo.join("data"))?;
        let table = profile.encoding_table(|p| read(repo, p))?;
        let spine = data
            .spines
            .get(&profile.spine)
            .ok_or_else(|| Error::new(format!("no spine {}", profile.spine)))?;
        if table.spine != spine.id {
            return Err(Error::new("encoding table is for another spine"));
        }
        let sel = Selection {
            riwayah: profile.riwayah.clone(),
            ..Selection::default()
        };
        let mut base = resolve_profile(&data, &sel, None, Mode::Data)
            .map_err(|e| Error::new(format!("resolve: {e:?}")))?;
        if hijazi_from_maghribi(&profile) {
            base = mushaf_resolve::maghribi_to_hijazi(&base, spine);
        }
        let basmalah = basmalah_words(&base, &table)?;
        let bytes = fs::read(repo.join(&profile.font))
            .map_err(|e| Error::new(format!("{}: {e}", profile.font)))?;
        Ok(Renderer {
            repo: repo.to_path_buf(),
            widths: LineWidths::from_json(&read(repo, &profile.line_widths)?)?,
            names: SurahNames::from_json(&read(repo, &profile.surah_names)?)?,
            outlines: Outlines::new(&bytes)?,
            font: FontData::new(bytes)?,
            basmalah,
            base,
            exceptions: Exceptions::from_json(&read(repo, EXCEPTIONS)?)?,
            table,
            data,
            profile,
            regexps: Regexps::new(),
        })
    }

    /// Resolves through the riwāyah's profile (T5.1). An overlay (free edits) resolves in free
    /// mode.
    pub fn resolve(&self, overlay: Option<&EditOverlay>) -> Result<Resolved, Error> {
        self.resolve_with(&BTreeMap::new(), overlay)
    }

    /// Resolves with option choices (T6.2) by group id; groups not named take their defaults.
    pub fn resolve_with(
        &self,
        choices: &BTreeMap<String, String>,
        overlay: Option<&EditOverlay>,
    ) -> Result<Resolved, Error> {
        let sel = Selection {
            riwayah: self.profile.riwayah.clone(),
            tariq: None,
            choices: choices.clone(),
        };
        let mode = if overlay.is_some() {
            Mode::Free
        } else {
            Mode::Data
        };
        let r = resolve_profile(&self.data, &sel, overlay, mode)
            .map_err(|e| Error::new(format!("resolve: {e:?}")))?;
        Ok(self.restyle(r))
    }

    /// D46: a Maghribi riwāyah in a Hijazi profile is written in Hijazi conventions.
    fn restyle(&self, r: Resolved) -> Resolved {
        if hijazi_from_maghribi(&self.profile) {
            mushaf_resolve::maghribi_to_hijazi(&r, &self.data.spines[&self.profile.spine])
        } else {
            r
        }
    }

    pub fn map(&self, resolved: &Resolved) -> Result<Mapping, Error> {
        let spine = &self.data.spines[&self.profile.spine];
        let layout = self
            .data
            .layouts
            .get(&self.profile.layout_id)
            .ok_or_else(|| Error::new(format!("no layout {}", self.profile.layout_id)))?;
        match &layout.riwayah {
            None => map_lines(layout, spine, resolved),
            Some(r) if *r == self.profile.riwayah => map_lines_on(layout, &self.base, resolved),
            Some(r) => {
                return Err(Error::new(format!(
                    "layout {} is for {r}, not {}",
                    layout.layout_id, self.profile.riwayah
                )));
            }
        }
        .map_err(|e| Error::new(e.to_string()))
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

/// Fit a page and check every word against rule 6 (with the owner's exceptions).
pub fn fit_checked(
    r: &Renderer,
    shaper: &mushaf_shape::Shaper,
    resolved: &Resolved,
    mapping: &Mapping,
    page: u32,
) -> Result<CheckedPage, Error> {
    let fit = match fit_with(r, shaper, resolved, mapping, page) {
        Ok(f) => f,
        Err(e) if e.to_string().contains("no encoding row") => {
            return Ok(CheckedPage {
                fit: None,
                blocked: Some(e.to_string()),
                violations: Vec::new(),
            });
        }
        Err(e) => return Err(e),
    };
    let mut violations = Vec::new();
    for l in &fit.lines {
        for w in &l.words {
            for v in check_word(&r.font, shaper, &w.chars, &w.features, &w.glyphs)? {
                if !r.exceptions.allows(&r.profile.font, &v) {
                    let at = w
                        .item
                        .map(|i| resolved.items[i].id.to_string())
                        .unwrap_or_else(|| format!("{page}:{} header", l.line));
                    violations.push((at, v));
                }
            }
        }
    }
    Ok(CheckedPage {
        fit: Some(fit),
        blocked: None,
        violations,
    })
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

/// Which differences from Ḥafṣ to highlight (T6.11, D37, D45).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighlightPolicy {
    /// Also highlight uncited rules (hypotheses), flagged `cited: false` for a distinct style.
    /// On by default: the owner chose to show them (2026-10-26, D45).
    pub hypotheses: bool,
}

impl Default for HighlightPolicy {
    fn default() -> Self {
        HighlightPolicy { hypotheses: true }
    }
}

impl HighlightPolicy {
    /// Only cited rules and curated farsh.
    pub const CITED_ONLY: HighlightPolicy = HighlightPolicy { hypotheses: false };
}

/// The highlights of a drawn page (T6.11–T6.12, D45). A highlight is an application of a
/// rule, or of a curated farsh patch (`source.kind: farsh`), on a cluster drawn on the page;
/// never a raw or residual patch, never an `encoding` or `spelling` rule, and nothing on
/// Ḥafṣ itself (highlights compare with Ḥafṣ).
pub fn highlights(
    data: &mushaf_model::DataSet,
    spine: &mushaf_model::Spine,
    resolved: &Resolved,
    page: &DrawPage,
    policy: HighlightPolicy,
) -> Vec<mushaf_draw::Highlight> {
    use mushaf_model::{DifferenceKind, RuleStatus};
    if page.riwayah == "hafs" {
        return Vec::new();
    }
    let mut line_of: BTreeMap<&mushaf_model::ClusterRef, u32> = BTreeMap::new();
    for it in &page.items {
        if let mushaf_draw::DrawItem::Glyph {
            line,
            cluster: Some(c),
            ..
        } = it
        {
            line_of.entry(c).or_insert(*line);
        }
    }
    let farsh: BTreeMap<&str, &mushaf_model::Patch> = data
        .patch_files
        .values()
        .flat_map(|f| &f.patches)
        .filter(|p| p.source.kind == "farsh")
        .map(|p| (p.id.as_str(), p))
        .collect();
    let mut out = Vec::new();
    for item in &resolved.items {
        for c in &item.clusters {
            let Some(line) = line_of.get(&c.id) else {
                continue;
            };
            let hafs_cps = spine.cluster(&c.id).map(|h| h.cps.clone());
            let hafs = hafs_cps
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            let annotation = hafs_cps.as_ref() == Some(&c.cps);
            for pid in &c.provenance {
                let found = match pid.split_once('@') {
                    Some((rule, _)) => data.register.entry(rule).and_then(|e| {
                        let kind_ok = !matches!(
                            e.difference_kind,
                            Some(DifferenceKind::Encoding | DifferenceKind::Spelling)
                        );
                        let cited = e.status == RuleStatus::RuleDerived && e.source.is_some();
                        let cite = e.source.as_ref().map(|s| match s.volume {
                            Some(v) => format!("{}, vol {v}, p. {}", s.book, s.page),
                            None => format!("{}, p. {}", s.book, s.page),
                        });
                        (kind_ok && (cited || policy.hypotheses))
                            .then(|| (rule.to_string(), cite, cited))
                    }),
                    None => farsh
                        .get(pid.as_str())
                        .map(|p| (p.id.clone(), Some(p.source.reference.clone()), true)),
                };
                if let Some((label, cite, cited)) = found {
                    out.push(mushaf_draw::Highlight {
                        line: *line,
                        cluster: c.id.clone(),
                        label,
                        hafs: hafs.clone(),
                        cite,
                        cited,
                        annotation,
                    });
                    break;
                }
            }
        }
    }
    out
}

impl Renderer {
    /// Resolve, fit and draw one page, then attach its highlights (T6.12). Everything drawn is
    /// the same as `render` with the same choices: only `DrawPage::highlights` is added.
    pub fn render_highlighted(
        &self,
        page: u32,
        choices: &BTreeMap<String, String>,
        policy: HighlightPolicy,
    ) -> Result<DrawPage, Error> {
        let resolved = self.resolve_with(choices, None)?;
        let mapping = self.map(&resolved)?;
        let mut d = self.draw(&self.fit(&resolved, &mapping, page)?)?;
        d.highlights = highlights(
            &self.data,
            &self.data.spines[&self.profile.spine],
            &resolved,
            &d,
            policy,
        );
        Ok(d)
    }
}
