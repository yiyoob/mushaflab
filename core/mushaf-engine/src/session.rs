//! A reader's session over one riwāyah in one style (D56): option choices, the free-edit
//! session (Phase 7) and caches, and everything the apps show besides the page itself: option
//! groups, margin khilāf notes (D60), the engine inspector and the provenance summary. The
//! apps only draw what these return (rule 7).

use std::collections::BTreeMap;

use mushaf_draw::{DrawItem, DrawPage, PathOp};
use mushaf_layout::Mapping;
use mushaf_layout::fit::{FitCache, FitNote};
use mushaf_model::{
    Attrs, ClusterRef, Cp, EditOverlay, Error, ItemRef, NewCluster, NewItem, NewNode, Ref,
    RuleStatus, Side, Status,
};
use mushaf_resolve::{Diagnostic, EditLint, EditSession, Origin, Resolved};
use serde::{Deserialize, Serialize};

use crate::{HighlightPolicy, Renderer, Source, highlights};

/// The author recorded on patches made in the web editor.
pub const AUTHOR: &str = "owner";

type Choices = BTreeMap<String, String>;

/// How many drawn pages a session keeps (T8.4).
pub const PAGE_CACHE: usize = 12;

struct Current {
    choices: Choices,
    overlay: Option<EditOverlay>,
    resolved: Resolved,
    mapping: Mapping,
}

/// An owner approval (rule 4), read from `data/approvals/<riwāyah>.yaml`; shown, never written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub target_ref: String,
    pub status: String,
    pub by: String,
    pub date: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Deserialize)]
struct ApprovalFile {
    #[serde(default)]
    approvals: Vec<Approval>,
}

pub struct Session {
    pub renderer: Renderer,
    pub style: String,
    choices: Choices,
    edits: EditSession,
    current: Option<Current>,
    fit_cache: FitCache,
    /// The margin's differences for a set of choices (D60).
    margin: Option<(Choices, Vec<MarginDiff>)>,
    /// Pages drawn for the current choices and edits, by (page, highlighted) (T8.4).
    pages: BTreeMap<(u32, bool), DrawPage>,
    approvals: Vec<Approval>,
}

/// An option group the reader can set (T6.2): only groups the data defines for this riwāyah.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OptionGroupView {
    pub id: String,
    pub title_ar: String,
    pub choices: Vec<String>,
    /// From the data (the resolver's choice with nothing set, else the group's default for
    /// this riwāyah); never a value of the app's own.
    pub default: Option<String>,
    pub current: Option<String>,
}

/// A word drawn by the core for the margin (font units, y up, glyphs left to right).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DrawWord {
    pub upem: i64,
    pub advance: i64,
    pub glyphs: BTreeMap<u32, Vec<PathOp>>,
    pub placed: Vec<Placed>,
}

impl DrawWord {
    /// The word as a standalone SVG (black on transparent), sized to its ink, from the same
    /// path writer as the page (`mushaf_draw::svg_path_d`).
    pub fn to_svg(&self) -> String {
        use mushaf_draw::fmt_26_6;
        let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for p in &self.placed {
            for op in &self.glyphs[&p.glyph] {
                let pts: &[(i32, i32)] = &match *op {
                    PathOp::Move { x, y } | PathOp::Line { x, y } => vec![(x, y)],
                    PathOp::Quad { x1, y1, x, y } => vec![(x1, y1), (x, y)],
                    PathOp::Cubic {
                        x1,
                        y1,
                        x2,
                        y2,
                        x,
                        y,
                    } => vec![(x1, y1), (x2, y2), (x, y)],
                    PathOp::Close => vec![],
                };
                for &(x, y) in pts {
                    let (x, y) = (i64::from(x) + p.x * 64, -(i64::from(y) + p.y * 64));
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                }
            }
        }
        if x0 > x1 {
            (x0, y0, x1, y1) = (0, 0, 64, 64);
        }
        let mut s = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\"><g fill=\"currentColor\">",
            fmt_26_6(x0),
            fmt_26_6(y0),
            fmt_26_6(x1 - x0),
            fmt_26_6(y1 - y0)
        );
        for p in &self.placed {
            s.push_str(&format!(
                "<path transform=\"matrix(1 0 0 -1 {} {})\" d=\"{}\"/>",
                fmt_26_6(p.x * 64),
                fmt_26_6(-p.y * 64),
                mushaf_draw::svg_path_d(&self.glyphs[&p.glyph])
            ));
        }
        s + "</g></svg>"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Placed {
    pub glyph: u32,
    pub x: i64,
    pub y: i64,
}

/// One cluster on the page that another wajh of an option group reads differently (D60):
/// its text, its annotations (D40), or both. Margins never change line breaks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MarginNote {
    pub group: String,
    pub title_ar: String,
    pub choice: String,
    pub line: u32,
    pub cluster: ClusterRef,
    /// The cluster under that choice, as `U+XXXX` codepoints; empty if it is deleted there.
    pub text: String,
    pub current_text: String,
    pub annotations: BTreeMap<String, String>,
    pub current_annotations: BTreeMap<String, String>,
    /// The whole word under that choice, drawn by the core; `None` if it cannot be converted
    /// to the font's encoding (reported, never guessed).
    pub word: Option<DrawWord>,
    pub word_error: Option<String>,
}

struct MarginDiff {
    group: String,
    title_ar: String,
    choice: String,
    cluster: ClusterRef,
    /// The item the cluster belongs with on the page.
    anchor: ItemRef,
    text: String,
    current_text: String,
    annotations: BTreeMap<String, String>,
    current_annotations: BTreeMap<String, String>,
    word: Option<DrawWord>,
    word_error: Option<String>,
}

/// One entry of a cluster's provenance with its status (rule 4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProvEntry {
    pub id: String,
    /// `patch`, `rule` or `user`.
    pub kind: String,
    /// As written in the data (`raw-derived`, `rule-derived`, `hypothesis`, `user-edit`).
    pub status: String,
    /// The owner's approval status where there is one, else `status`.
    pub effective_status: String,
    pub source: Option<String>,
    pub single_source: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InspectCluster {
    pub line: u32,
    pub id: ClusterRef,
    pub text: String,
    /// The Ḥafṣ spine's cluster; empty for an inserted cluster.
    pub hafs: String,
    #[serde(skip_serializing_if = "Attrs::is_empty")]
    pub attrs: Attrs,
    pub provenance: Vec<ProvEntry>,
}

/// The engine inspector (T8.3): what produced this page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Inspect {
    pub riwayah: String,
    pub style: String,
    pub profile: String,
    pub layout_id: String,
    pub font: String,
    pub spine: String,
    pub patch_stack: Vec<String>,
    pub rule_stack: Vec<String>,
    pub choices: Choices,
    pub user_patches: Vec<String>,
    pub report: Vec<Diagnostic>,
    pub canonical: bool,
    pub notes: Vec<FitNote>,
    pub clusters: Vec<InspectCluster>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PatchFileSummary {
    pub path: String,
    pub patches: usize,
    pub by_status: BTreeMap<String, usize>,
    pub single_source: usize,
    pub approved: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuleSummary {
    pub id: String,
    pub file: String,
    pub description: String,
    pub status: String,
    pub effective_status: String,
    pub source: Option<String>,
    pub single_source: bool,
    pub option: Option<String>,
}

/// The provenance and verification-status screen (T8.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Provenance {
    pub riwayah: String,
    pub style: String,
    pub text_source: String,
    pub single_source: bool,
    pub layout_id: String,
    pub layout_sources: BTreeMap<String, String>,
    pub font: String,
    pub patch_files: Vec<PatchFileSummary>,
    pub rules: Vec<RuleSummary>,
    pub approvals: Vec<Approval>,
}

/// A free edit from the editor (Phase 7 `EditSession`), as JSON.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditOp {
    Insert {
        anchor: String,
        after: bool,
        /// Exactly one of `item` (a new word) or `cluster` (a new letter).
        item: Option<NewItem>,
        cluster: Option<NewCluster>,
    },
    Delete {
        target: String,
    },
    ReplaceCluster {
        target: String,
        cluster: NewCluster,
    },
    AddMark {
        target: String,
        cp: Cp,
        at: Option<u32>,
    },
    RemoveMark {
        target: String,
        cp: Cp,
    },
    SetWidth {
        target: String,
        value: Option<i32>,
    },
    SetStretch {
        target: String,
        value: Option<i32>,
    },
    Undo,
    Redo,
    Clear,
}

fn status_name(s: Status) -> &'static str {
    match s {
        Status::RawDerived => "raw-derived",
        Status::RuleDerived => "rule-derived",
        Status::UserEdit => "user-edit",
    }
}

fn rule_status_name(s: RuleStatus) -> &'static str {
    match s {
        RuleStatus::RuleDerived => "rule-derived",
        RuleStatus::Hypothesis => "hypothesis",
    }
}

fn cite(c: &mushaf_model::Citation) -> String {
    match c.volume {
        Some(v) => format!("{}, vol {v}, p. {}", c.book, c.page),
        None => format!("{}, p. {}", c.book, c.page),
    }
}

fn parse<T: std::str::FromStr<Err = Error>>(s: &str) -> Result<T, Error> {
    s.parse()
}

fn edit_err(e: mushaf_resolve::EditError) -> Error {
    Error::new(e.to_string())
}

/// The line each cluster drawn on the page sits on.
fn lines_of(d: &DrawPage) -> BTreeMap<ClusterRef, u32> {
    let mut out = BTreeMap::new();
    for it in &d.items {
        if let DrawItem::Glyph {
            line,
            cluster: Some(c),
            ..
        } = it
        {
            out.entry(c.clone()).or_insert(*line);
        }
    }
    out
}

impl Session {
    pub fn new<S: Source + ?Sized>(src: &S, renderer: Renderer, style: &str) -> Self {
        let approvals = src
            .read(&format!("data/approvals/{}.yaml", renderer.profile.riwayah))
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|t| mushaf_model::from_yaml::<ApprovalFile>(&t).ok())
            .map(|f| f.approvals)
            .unwrap_or_default();
        let spine = renderer.profile.spine.clone();
        Session {
            renderer,
            style: style.to_string(),
            choices: Choices::new(),
            edits: EditSession::new(spine, AUTHOR),
            current: None,
            fit_cache: FitCache::default(),
            margin: None,
            pages: BTreeMap::new(),
            approvals,
        }
    }

    pub fn pages(&self) -> u32 {
        self.renderer
            .data
            .layouts
            .get(&self.renderer.profile.layout_id)
            .map_or(0, |l| l.pages)
    }

    fn overlay(&self) -> Option<EditOverlay> {
        let o = self.edits.overlay();
        (!o.patches.is_empty()).then(|| o.clone())
    }

    fn ensure(&mut self) -> Result<(), Error> {
        let overlay = self.overlay();
        if let Some(c) = &self.current
            && c.choices == self.choices
            && c.overlay == overlay
        {
            return Ok(());
        }
        let resolved = self
            .renderer
            .resolve_with(&self.choices, overlay.as_ref())?;
        let mapping = self.renderer.map(&resolved)?;
        self.pages.clear();
        self.current = Some(Current {
            choices: self.choices.clone(),
            overlay,
            resolved,
            mapping,
        });
        Ok(())
    }

    /// The current text (resolved with the choices and the edits).
    pub fn resolved(&mut self) -> Result<&Resolved, Error> {
        self.ensure()?;
        Ok(&self.current.as_ref().expect("ensured").resolved)
    }

    /// The page, byte-identical to `Renderer::render` (plain) or `render_highlighted` with the
    /// same choices and edits. Highlights never change what is drawn (D45).
    pub fn render(&mut self, page: u32, highlighted: bool) -> Result<DrawPage, Error> {
        self.ensure()?;
        if let Some(d) = self.pages.get(&(page, highlighted)) {
            return Ok(d.clone());
        }
        let Session {
            renderer,
            current,
            fit_cache,
            pages,
            ..
        } = self;
        let cur = current.as_ref().expect("ensured");
        let fit = renderer.fit_cached(&cur.resolved, &cur.mapping, page, fit_cache)?;
        let mut d = renderer.draw(&fit)?;
        if highlighted {
            d.highlights = highlights(
                &renderer.data,
                &renderer.data.spines[&renderer.profile.spine],
                &cur.resolved,
                &d,
                HighlightPolicy::default(),
            );
        }
        // Keep the pages nearest this one (a reader pages back and forth around it).
        while pages.len() >= PAGE_CACHE {
            let far = *pages
                .keys()
                .max_by_key(|(p, h)| (p.abs_diff(page), *p, *h))
                .expect("not empty");
            pages.remove(&far);
        }
        pages.insert((page, highlighted), d.clone());
        Ok(d)
    }

    /// The option groups of this riwāyah's data, with defaults from the data.
    pub fn option_groups(&self) -> Vec<OptionGroupView> {
        let r = &self.renderer;
        let riwayah = &r.profile.riwayah;
        r.data
            .option_groups
            .values()
            .filter(|g| r.base.choices.contains_key(&g.id) || g.allowed_for.contains(riwayah))
            .map(|g| {
                let default = r.base.choices.get(&g.id).cloned().or_else(|| {
                    g.defaults
                        .iter()
                        .find(|d| d.riwayah == *riwayah && d.tariq.is_none())
                        .map(|d| d.choice.clone())
                });
                OptionGroupView {
                    id: g.id.clone(),
                    title_ar: g.title_ar.clone(),
                    choices: g.choices.clone(),
                    current: self.choices.get(&g.id).cloned().or_else(|| default.clone()),
                    default,
                }
            })
            .collect()
    }

    pub fn choices(&self) -> &Choices {
        &self.choices
    }

    /// Sets the reader's choices; groups not named take their defaults. A group or choice the
    /// data does not define is refused (options are data, not free edits).
    pub fn set_choices(&mut self, choices: Choices) -> Result<(), Error> {
        for (g, c) in &choices {
            let group = self
                .renderer
                .data
                .option_groups
                .get(g)
                .ok_or_else(|| Error::new(format!("no option group {g:?}")))?;
            if !group.choices.contains(c) {
                return Err(Error::new(format!("{g}: no choice {c:?}")));
            }
        }
        self.choices = choices;
        Ok(())
    }

    /// The core-drawn form of a word for the margin.
    pub fn draw_word(&self, clusters: &[&[char]]) -> Result<DrawWord, Error> {
        let r = &self.renderer;
        let conv = r
            .table
            .convert(clusters)
            .map_err(|u| Error::new(format!("no encoding row: {u:?}")))?;
        let shaped = r.font.shaper().shape(&conv.chars, &[])?;
        let mut glyphs = BTreeMap::new();
        let mut placed = Vec::new();
        let mut x = 0i64;
        for g in &shaped {
            glyphs
                .entry(g.glyph)
                .or_insert(r.outlines.get(g.glyph)?.to_vec());
            placed.push(Placed {
                glyph: g.glyph,
                x: x + i64::from(g.x_offset),
                y: i64::from(g.y_offset),
            });
            x += i64::from(g.x_advance);
        }
        Ok(DrawWord {
            upem: i64::from(r.font.units_per_em()),
            advance: x,
            glyphs,
            placed,
        })
    }

    /// Every cluster of the text that another wajh reads differently (D60), for the current
    /// choices; built once per set of choices. Edits are not compared: the margin shows the
    /// data's khilāf, not the user's changes.
    fn margin_index(&mut self) -> Result<&[MarginDiff], Error> {
        if self
            .margin
            .as_ref()
            .is_some_and(|(c, _)| *c == self.choices)
        {
            return Ok(&self.margin.as_ref().expect("checked").1);
        }
        let r = &self.renderer;
        let spine = &r.data.spines[&r.profile.spine];
        let current = r.resolve_with(&self.choices, None)?;
        let index = |res: &Resolved| -> BTreeMap<ClusterRef, (usize, usize)> {
            let mut m = BTreeMap::new();
            for (i, item) in res.items.iter().enumerate() {
                for (k, c) in item.clusters.iter().enumerate() {
                    m.insert(c.id.clone(), (i, k));
                }
            }
            m
        };
        let cur_ix = index(&current);
        let mut diffs = Vec::new();
        for g in self.option_groups() {
            let Some(cur_choice) = g.current.clone() else {
                continue;
            };
            for choice in g.choices.iter().filter(|c| **c != cur_choice) {
                let mut alt_choices = self.choices.clone();
                alt_choices.insert(g.id.clone(), choice.clone());
                let alt = r.resolve_with(&alt_choices, None)?;
                let alt_ix = index(&alt);
                let mut push =
                    |cluster: &ClusterRef,
                     anchor: ItemRef,
                     cur: Option<&mushaf_resolve::RCluster>,
                     alt_item: Option<&mushaf_resolve::RItem>| {
                        let alt_c =
                            alt_item.and_then(|i| i.clusters.iter().find(|c| c.id == *cluster));
                        let (word, word_error) = match alt_item {
                            Some(i) => {
                                let cl: Vec<&[char]> =
                                    i.clusters.iter().map(|c| c.cps.0.as_slice()).collect();
                                match self.draw_word(&cl) {
                                    Ok(w) => (Some(w), None),
                                    Err(e) => (None, Some(e.to_string())),
                                }
                            }
                            None => (None, None),
                        };
                        diffs.push(MarginDiff {
                            group: g.id.clone(),
                            title_ar: g.title_ar.clone(),
                            choice: choice.clone(),
                            cluster: cluster.clone(),
                            anchor,
                            text: alt_c.map(|c| c.cps.to_string()).unwrap_or_default(),
                            current_text: cur.map(|c| c.cps.to_string()).unwrap_or_default(),
                            annotations: alt_c
                                .map(|c| c.attrs.annotations.clone())
                                .unwrap_or_default(),
                            current_annotations: cur
                                .map(|c| c.attrs.annotations.clone())
                                .unwrap_or_default(),
                            word,
                            word_error,
                        });
                    };
                for item in &alt.items {
                    for c in &item.clusters {
                        let cur = cur_ix
                            .get(&c.id)
                            .map(|&(i, k)| &current.items[i].clusters[k]);
                        if cur.is_some_and(|k| k.cps == c.cps && k.attrs == c.attrs) {
                            continue;
                        }
                        // a cluster only this wajh has sits with the item it is inserted at
                        let anchor = match (cur_ix.get(&c.id), &item.origin) {
                            (Some(&(i, _)), _) => current.items[i].id.clone(),
                            (None, Origin::Inserted { anchor }) => spine
                                .items
                                .get(*anchor)
                                .map_or(item.id.clone(), |s| s.id.clone()),
                            (None, Origin::Spine { .. }) => item.id.clone(),
                        };
                        push(&c.id, anchor, cur, Some(item));
                    }
                }
                for item in &current.items {
                    for c in &item.clusters {
                        if !alt_ix.contains_key(&c.id) {
                            push(&c.id, item.id.clone(), Some(c), None);
                        }
                    }
                }
            }
        }
        self.margin = Some((self.choices.clone(), diffs));
        Ok(&self.margin.as_ref().expect("set").1)
    }

    /// Margin khilāf notes for the page (D60): for every option group and every choice other
    /// than the current one, each cluster on the page whose text or annotations differ. A
    /// cluster only the other wajh has is placed at the item it is inserted at. Empty where the
    /// data has no alternative wajh on the page (G-151).
    pub fn margin_notes(&mut self, page: u32) -> Result<Vec<MarginNote>, Error> {
        let d = self.render(page, false)?;
        let on_page = lines_of(&d);
        let item_line: BTreeMap<ItemRef, u32> =
            on_page.iter().map(|(c, l)| (c.item.clone(), *l)).collect();
        let mut out: Vec<MarginNote> = self
            .margin_index()?
            .iter()
            .filter_map(|m| {
                let line = on_page
                    .get(&m.cluster)
                    .or_else(|| item_line.get(&m.anchor))?;
                Some(MarginNote {
                    group: m.group.clone(),
                    title_ar: m.title_ar.clone(),
                    choice: m.choice.clone(),
                    line: *line,
                    cluster: m.cluster.clone(),
                    text: m.text.clone(),
                    current_text: m.current_text.clone(),
                    annotations: m.annotations.clone(),
                    current_annotations: m.current_annotations.clone(),
                    word: m.word.clone(),
                    word_error: m.word_error.clone(),
                })
            })
            .collect();
        out.sort_by(|a, b| (a.line, &a.cluster, &a.group).cmp(&(b.line, &b.cluster, &b.group)));
        Ok(out)
    }

    fn approval(&self, id: &str) -> Option<&Approval> {
        self.approvals.iter().find(|a| a.target_ref == id)
    }

    fn prov_entry(&self, pid: &str) -> ProvEntry {
        let data = &self.renderer.data;
        let effective = |status: &str| {
            self.approval(pid)
                .map_or(status.to_string(), |a| a.status.clone())
        };
        if let Some((rule, _)) = pid.split_once('@') {
            let e = data.register.entry(rule);
            let status = e.map_or("unregistered", |e| rule_status_name(e.status));
            return ProvEntry {
                id: pid.to_string(),
                kind: "rule".into(),
                effective_status: self
                    .approval(rule)
                    .map_or(status.to_string(), |a| a.status.clone()),
                status: status.into(),
                source: e.and_then(|e| e.source.as_ref()).map(cite),
                single_source: e.is_some_and(|e| e.single_source),
            };
        }
        if let Some(p) = self.edits.overlay().patches.iter().find(|p| p.id == pid) {
            return ProvEntry {
                id: pid.to_string(),
                kind: "user".into(),
                status: status_name(p.status).into(),
                effective_status: status_name(p.status).into(),
                source: Some(format!("{}: {}", p.source.kind, p.source.reference)),
                single_source: p.single_source,
            };
        }
        let found = self
            .renderer
            .profile_patch_files()
            .flat_map(|f| f.patches.iter())
            .find(|p| p.id == pid);
        match found {
            Some(p) => ProvEntry {
                id: pid.to_string(),
                kind: "patch".into(),
                status: status_name(p.status).into(),
                effective_status: effective(status_name(p.status)),
                source: Some(format!("{}: {}", p.source.kind, p.source.reference)),
                single_source: p.single_source,
            },
            None => ProvEntry {
                id: pid.to_string(),
                kind: "patch".into(),
                status: "unknown".into(),
                effective_status: effective("unknown"),
                source: None,
                single_source: false,
            },
        }
    }

    /// The engine inspector for a page: stacks, choices, report, and every drawn cluster with
    /// its provenance.
    pub fn inspect(&mut self, page: u32) -> Result<Inspect, Error> {
        let d = self.render(page, false)?;
        let on_page = lines_of(&d);
        let resolved = self.resolved()?.clone();
        let p = &self.renderer.profile;
        let rp = &self.renderer.data.profiles[&p.riwayah];
        let spine = &self.renderer.data.spines[&p.spine];
        let mut clusters = Vec::new();
        for item in &resolved.items {
            for c in &item.clusters {
                let Some(line) = on_page.get(&c.id) else {
                    continue;
                };
                clusters.push(InspectCluster {
                    line: *line,
                    id: c.id.clone(),
                    text: c.cps.to_string(),
                    hafs: spine
                        .cluster(&c.id)
                        .map(|h| h.cps.to_string())
                        .unwrap_or_default(),
                    attrs: c.attrs.clone(),
                    provenance: c.provenance.iter().map(|id| self.prov_entry(id)).collect(),
                });
            }
        }
        Ok(Inspect {
            riwayah: p.riwayah.clone(),
            style: self.style.clone(),
            profile: p.profile.clone(),
            layout_id: p.layout_id.clone(),
            font: p.font.clone(),
            spine: p.spine.clone(),
            patch_stack: rp.patch_stack.clone(),
            rule_stack: rp.rule_stack.clone(),
            choices: resolved.choices.clone(),
            user_patches: resolved.user_patches.clone(),
            report: resolved.report.clone(),
            canonical: d.canonical,
            notes: d.notes.clone(),
            clusters,
        })
    }

    /// The provenance and verification-status summary of this riwāyah's stack.
    pub fn provenance(&self) -> Provenance {
        let r = &self.renderer;
        let p = &r.profile;
        let rp = &r.data.profiles[&p.riwayah];
        let layout = &r.data.layouts[&p.layout_id];
        let patch_files = rp
            .patch_stack
            .iter()
            .filter_map(|path| r.data.patch_files.get(path).map(|f| (path, f)))
            .map(|(path, f)| {
                let mut by_status = BTreeMap::new();
                for x in &f.patches {
                    *by_status
                        .entry(status_name(x.status).to_string())
                        .or_insert(0) += 1;
                }
                PatchFileSummary {
                    path: path.clone(),
                    patches: f.patches.len(),
                    by_status,
                    single_source: f.patches.iter().filter(|x| x.single_source).count(),
                    approved: f
                        .patches
                        .iter()
                        .filter(|x| self.approval(&x.id).is_some())
                        .count(),
                }
            })
            .collect();
        let mut rules = Vec::new();
        for path in &rp.rule_stack {
            let Some(f) = r.data.rule_files.get(path) else {
                continue;
            };
            for rule in &f.rules {
                let e = r.data.register.entry(&rule.id);
                let status = e.map_or("unregistered", |e| rule_status_name(e.status));
                rules.push(RuleSummary {
                    id: rule.id.clone(),
                    file: path.clone(),
                    description: e.map(|e| e.description.clone()).unwrap_or_default(),
                    effective_status: self
                        .approval(&rule.id)
                        .map_or(status.to_string(), |a| a.status.clone()),
                    status: status.into(),
                    source: e.and_then(|e| e.source.as_ref()).map(cite),
                    single_source: rule.single_source,
                    option: e.and_then(|e| e.option.clone()),
                });
            }
        }
        Provenance {
            riwayah: p.riwayah.clone(),
            style: self.style.clone(),
            text_source: rp.text_source.clone(),
            single_source: rp.single_source,
            layout_id: p.layout_id.clone(),
            layout_sources: layout.sources.clone(),
            font: p.font.clone(),
            patch_files,
            rules,
            approvals: self.approvals.clone(),
        }
    }

    /// Applies one free edit (rule 11: never refused for being non-canonical; refused only
    /// when the target does not exist or the edit is malformed).
    pub fn edit(&mut self, op: EditOp) -> Result<(), Error> {
        self.ensure()?;
        let cur = &self.current.as_ref().expect("ensured").resolved;
        let e = &mut self.edits;
        match op {
            EditOp::Insert {
                anchor,
                after,
                item,
                cluster,
            } => {
                let node = match (item, cluster) {
                    (Some(i), None) => NewNode::Item(i),
                    (None, Some(c)) => NewNode::Cluster(c),
                    _ => return Err(Error::new("insert: give exactly one of item or cluster")),
                };
                let side = if after { Side::After } else { Side::Before };
                e.insert(cur, parse::<Ref>(&anchor)?, side, node)
                    .map(|_| ())
                    .map_err(edit_err)
            }
            EditOp::Delete { target } => e.delete(cur, parse(&target)?).map_err(edit_err),
            EditOp::ReplaceCluster { target, cluster } => e
                .replace_cluster(cur, parse(&target)?, cluster)
                .map_err(edit_err),
            EditOp::AddMark { target, cp, at } => {
                e.add_mark(cur, parse(&target)?, cp, at).map_err(edit_err)
            }
            EditOp::RemoveMark { target, cp } => {
                e.remove_mark(cur, parse(&target)?, cp).map_err(edit_err)
            }
            EditOp::SetWidth { target, value } => {
                e.set_width(cur, parse(&target)?, value).map_err(edit_err)
            }
            EditOp::SetStretch { target, value } => {
                e.set_stretch(cur, parse(&target)?, value).map_err(edit_err)
            }
            EditOp::Undo => {
                e.undo();
                Ok(())
            }
            EditOp::Redo => {
                e.redo();
                Ok(())
            }
            EditOp::Clear => {
                *e = EditSession::new(self.renderer.profile.spine.clone(), AUTHOR);
                Ok(())
            }
        }
    }

    /// The number of patches in the edit overlay.
    pub fn edit_count(&self) -> usize {
        self.edits.overlay().patches.len()
    }

    pub fn can_undo(&self) -> bool {
        self.edits.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.edits.can_redo()
    }

    pub fn export_edits(&self) -> Result<String, Error> {
        self.edits.export().map_err(edit_err)
    }

    /// Replaces the edits with an exported overlay (T7.2).
    pub fn import_edits(&mut self, text: &str) -> Result<(), Error> {
        self.edits =
            EditSession::import(&self.renderer.profile.spine, AUTHOR, text).map_err(edit_err)?;
        Ok(())
    }

    /// T7.3 lint of the edits (advisory only, D54).
    pub fn lint(&self) -> Result<Vec<EditLint>, Error> {
        match self.overlay() {
            Some(o) => self.renderer.lint(&self.choices, &o),
            None => Ok(Vec::new()),
        }
    }
}

impl Renderer {
    /// The patch files of this riwāyah's profile stack.
    pub fn profile_patch_files(&self) -> impl Iterator<Item = &mushaf_model::PatchFile> {
        self.data.profiles[&self.profile.riwayah]
            .patch_stack
            .iter()
            .filter_map(|p| self.data.patch_files.get(p))
    }
}
