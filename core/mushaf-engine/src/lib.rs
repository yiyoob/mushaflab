//! The page engine (D56): loads a render profile and its data through a `Source` and runs the
//! core end to end: resolve, map lines, fit (shape + justify), draw, highlight. It does no I/O
//! of its own, so the CLI (files on disk) and the web app (a static bundle, D57) run the same
//! code and get byte-identical draw lists.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mushaf_draw::{DrawPage, Outlines, PageMeta, draw_page};
use mushaf_justify::Regexps;
use mushaf_layout::fit::{
    FitCache, FitData, FitPage, LineWidths, Profile, SurahNames, basmalah_words, fit_page,
    fit_page_cached,
};
use mushaf_layout::{Mapping, map_lines, map_lines_on};
use mushaf_model::{DataFiles, DataSet, EditOverlay, Error};
use mushaf_resolve::{EditLint, Mode, Resolved, Selection, lint_overlay, resolve_profile};
use mushaf_shape::{EncodingTable, Exceptions, FontData, Violation, check_word};

/// Where the engine reads its inputs: files by repository-relative path (`data/render/...`,
/// `fonts/...`) and the data root's files (`DataSet::from_files`). A directory on disk for the
/// CLI; the static bundle on the web (D57).
pub trait Source {
    fn read(&self, rel: &str) -> Result<Vec<u8>, Error>;
    /// Every file under the data root's data directories, keyed relative to `data/`.
    fn data_files(&self) -> Result<DataFiles, Error>;
}

/// A repository checkout on disk.
impl Source for Path {
    fn read(&self, rel: &str) -> Result<Vec<u8>, Error> {
        std::fs::read(self.join(rel)).map_err(|e| Error::new(format!("{rel}: {e}")))
    }
    fn data_files(&self) -> Result<DataFiles, Error> {
        mushaf_model::read_tree(&self.join("data"))
    }
}

impl Source for PathBuf {
    fn read(&self, rel: &str) -> Result<Vec<u8>, Error> {
        self.as_path().read(rel)
    }
    fn data_files(&self) -> Result<DataFiles, Error> {
        self.as_path().data_files()
    }
}

pub mod bundle;
pub mod session;

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
pub fn check_style_available<S: Source + ?Sized>(
    repo: &S,
    riwayah: &str,
    style: &str,
) -> Result<(), Error> {
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

/// One (riwāyah, style) combination a reader can pick (D36, D44): every riwāyah's default
/// style, and each non-default style the T6.9 audit marks available. Nothing else is offered.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct StyleOption {
    pub riwayah: String,
    pub style: String,
    pub default: bool,
}

/// The combinations to offer, riwāyāt in the order of their profiles in the data.
pub fn styles<S: Source + ?Sized>(repo: &S, data: &DataSet) -> Result<Vec<StyleOption>, Error> {
    let mut out = Vec::new();
    for riwayah in data.profiles.keys() {
        let default = default_style(riwayah)?;
        for style in ["hijazi", "maghribi"] {
            let ok = style == default || check_style_available(repo, riwayah, style).is_ok();
            if ok {
                out.push(StyleOption {
                    riwayah: riwayah.clone(),
                    style: style.into(),
                    default: style == default,
                });
            }
        }
    }
    Ok(out)
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
    pub profile: Profile,
    /// Shared: every riwāyah of a session reads the same data set (D57).
    pub data: Arc<DataSet>,
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

fn read<S: Source + ?Sized>(repo: &S, rel: &str) -> Result<String, Error> {
    String::from_utf8(repo.read(rel)?).map_err(|e| Error::new(format!("{rel}: {e}")))
}

impl Renderer {
    /// The riwāyah in its default style (D36).
    pub fn load<S: Source + ?Sized>(repo: &S, riwayah: &str) -> Result<Self, Error> {
        Self::load_style(repo, riwayah, default_style(riwayah)?)
    }

    pub fn load_style<S: Source + ?Sized>(
        repo: &S,
        riwayah: &str,
        style: &str,
    ) -> Result<Self, Error> {
        let data = Arc::new(DataSet::from_files(&repo.data_files()?)?);
        Self::load_with(repo, data, riwayah, style)
    }

    /// `load_style` over a data set already parsed from the same source.
    pub fn load_with<S: Source + ?Sized>(
        repo: &S,
        data: Arc<DataSet>,
        riwayah: &str,
        style: &str,
    ) -> Result<Self, Error> {
        check_style_available(repo, riwayah, style)?;
        let profile = Profile::from_json(&read(repo, &profile_path(riwayah, style)?)?)?;
        if profile.riwayah != riwayah || profile.style != style {
            return Err(Error::new(format!(
                "render profile {} is for {} in style {}",
                profile.profile, profile.riwayah, profile.style
            )));
        }
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
        let bytes = repo.read(&profile.font)?;
        Ok(Renderer {
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

    /// `fit` reusing `cache` across calls (T7.4): after an edit only the changed lines are
    /// justified and the changed words shaped again. The cache belongs to this renderer's font.
    pub fn fit_cached(
        &self,
        resolved: &Resolved,
        mapping: &Mapping,
        page: u32,
        cache: &mut FitCache,
    ) -> Result<FitPage, Error> {
        fit_page_cached(
            &self.font.shaper(),
            &self.regexps,
            &self.fit_data(),
            resolved,
            mapping,
            page,
            cache,
        )
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

    /// T7.3 (D54): how an overlay departs from this riwāyah's canonical text under `choices`,
    /// item by item, with the other option choices each edited item matches. Advisory only.
    pub fn lint(
        &self,
        choices: &BTreeMap<String, String>,
        overlay: &EditOverlay,
    ) -> Result<Vec<EditLint>, Error> {
        lint_overlay(
            |c, o| self.resolve_with(c, o),
            choices,
            &self.data.option_groups,
            overlay,
        )
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
    use mushaf_model::RuleStatus;
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
    // Writing conventions are not differences (D46): a Maghribi text is compared in Hijazi
    // writing, and pause signs and the sukun sign's form are ignored.
    let written = if page.riwayah == "warsh" || page.riwayah == "qalun" {
        mushaf_resolve::maghribi_to_hijazi(resolved, spine)
    } else {
        resolved.clone()
    };
    // Sukun is ignored outright: Hafs leaves it off before an assimilated letter, the Maghribi
    // print writes it (owner, 2026-10-26: أَنْ is not a difference).
    let norm = |cps: &[char]| -> Vec<char> {
        cps.iter()
            .filter(|c| !('\u{06D6}'..='\u{06DB}').contains(*c))
            .filter(|c| !matches!(c, '\u{0652}' | '\u{06E1}'))
            // the three shapes of final ya are one letter (spelling, not reading)
            .map(|c| match c {
                '\u{0649}' | '\u{06D2}' => '\u{064A}',
                // hamza on alif written as alif + the same vowel (spelling); a lost vowel
                // (ibdal, tashil) still differs
                '\u{0623}' | '\u{0625}' => '\u{0627}',
                c => *c,
            })
            .collect()
    };
    // The Maghribi print writes a bare lam where Hafs has lam + shadda + fatha (ٱلَّذِي, لِلَّهِ;
    // 1,674 places in both Warsh and Qalun; owner, 2026-10-26: not a difference).
    // The Maghribi print also writes the shadda of idgham with ghunna on a word's first letter
    // after nun or tanwin, where Hafs leaves it off (مَن يَّقُولُ / مَن يَقُولُ): same reading.
    let same = |h: &[char], w: &[char], after_nun: bool| {
        let (h, w) = (norm(h), norm(w));
        let unshadda =
            |x: &[char]| -> Vec<char> { x.iter().filter(|c| **c != '\u{0651}').copied().collect() };
        h == w
            || (w == ['\u{0644}'] && h == ['\u{0644}', '\u{0651}', '\u{064E}'])
            || (after_nun && !h.contains(&'\u{0651}') && unshadda(&w) == h)
    };
    let ends_in_nun = |cps: &[char]| {
        let n = norm(cps);
        (n.first() == Some(&'\u{0646}') && n.len() == 1)
            || n.iter()
                .any(|c| matches!(c, '\u{064B}'..='\u{064D}' | '\u{08F0}'..='\u{08F2}'))
    };
    let mut prev_last: Option<Vec<char>> = None;
    let mut out = Vec::new();
    for item in written
        .items
        .iter()
        .filter(|i| i.kind == mushaf_model::ItemKind::Word)
    {
        // The disjoined letters (الٓمٓ …) carry vowels in the Maghribi print only: same reading.
        // In Hafs they are the words with a maddah and no vowel at all.
        let hafs_word: Vec<char> = item
            .clusters
            .iter()
            .filter_map(|c| spine.cluster(&c.id))
            .flat_map(|h| h.cps.0.iter().copied())
            .collect();
        let muqattaa = hafs_word.contains(&'\u{0653}')
            && !hafs_word
                .iter()
                .any(|c| matches!(c, '\u{064B}'..='\u{0650}' | '\u{08F0}'..='\u{08F2}'));
        let after_nun = prev_last.as_deref().is_some_and(ends_in_nun);
        prev_last = item
            .clusters
            .last()
            .and_then(|c| spine.cluster(&c.id))
            .map(|h| h.cps.0.clone());
        for (k, c) in item.clusters.iter().enumerate() {
            if muqattaa {
                break;
            }
            let Some(line) = line_of.get(&c.id) else {
                continue;
            };
            let hafs_cps = spine.cluster(&c.id).map(|h| h.cps.clone());
            if hafs_cps
                .as_ref()
                .is_some_and(|h| same(&h.0, &c.cps.0, k == 0 && after_nun))
            {
                continue;
            }
            let rule = c.provenance.iter().find_map(|pid| {
                let (rule, _) = pid.split_once('@')?;
                let e = data.register.entry(rule)?;
                let cited = e.status == RuleStatus::RuleDerived && e.source.is_some();
                let cite = e.source.as_ref().map(|s| match s.volume {
                    Some(v) => format!("{}, vol {v}, p. {}", s.book, s.page),
                    None => format!("{}, p. {}", s.book, s.page),
                });
                Some((rule.to_string(), cite, cited))
            });
            let (label, cite, cited) = match rule {
                Some(r) => r,
                None => (
                    c.provenance.first().cloned().unwrap_or_default(),
                    None,
                    false,
                ),
            };
            if !cited && !policy.hypotheses {
                continue;
            }
            out.push(mushaf_draw::Highlight {
                line: *line,
                cluster: c.id.clone(),
                label,
                hafs: hafs_cps
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                cite,
                cited,
                annotation: false,
            });
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
