//! Draw commands and the SVG backend (T3.3; D4, D16, rule 11).
//!
//! D4: the core returns positioned glyph outlines. A `DrawPage` holds each used glyph's outline
//! once (26.6 font units, y up) and a list of draw items in page coordinates (26.6 page units,
//! y down): glyphs placed with an origin and a 16.16 scale, and strokes (surah-header frames,
//! the "edited" marker) and fills (the sajdah overline, D18). Platforms only fill and stroke
//! paths. Every glyph carries the id of
//! the spine (or inserted) cluster it draws, for hit-testing and per-glyph edits.
//!
//! Rule 11: a non-canonical page (`FitPage::canonical == false`) always gets the edited
//! marker, a red frame and the word EDITED drawn as strokes, appended after everything else.
//! There is no option to leave it out.

mod outline;
mod svg;

use std::collections::BTreeSet;

use mushaf_justify::SCALE_ONE;
use mushaf_layout::fit::{FitKind, FitLine, FitNote, FitPage, Geometry};
use mushaf_model::{ClusterRef, Error};
use serde::Serialize;

pub use outline::{Outlines, PathOp};
pub use svg::{fmt_16_16, fmt_26_6, to_svg};

/// 26.6 unit.
const ONE: i64 = 64;
/// Baseline of a line, from the top of its slot, per mille of the pitch.
const BASELINE: i64 = 620;
/// Lines per full page (Madina 15-line layouts); shorter pages are centred vertically.
const LINES: i64 = 15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The box of a surah-header line.
    Frame,
    /// Rule 11: the page is not canonical.
    EditedMarker,
    /// D18: the bar over a sajdah word.
    SajdahOverline,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DrawItem {
    /// Fill glyph `glyph`'s outline, scaled by (`sx`, `-sy`) and moved to (`x`, `y`).
    Glyph {
        glyph: u32,
        x: i64,
        y: i64,
        sx: i64,
        sy: i64,
        line: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        cluster: Option<ClusterRef>,
    },
    /// Fill a path given in page units (26.6, y down).
    Fill {
        role: Role,
        line: u32,
        path: Vec<PathOp>,
    },
    /// Stroke a path given in page units (26.6, y down).
    Stroke {
        role: Role,
        width: i64,
        path: Vec<PathOp>,
    },
}

/// What a line was fitted to and where it was drawn (26.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LineSummary {
    pub line: u32,
    pub kind: FitKind,
    pub short_ppm: Option<i64>,
    /// Font units at the page's scale.
    pub target: i64,
    pub width: i64,
    pub natural: i64,
    pub squeeze: Option<(i64, i64)>,
    pub letter_gain: i64,
    /// Page units: the drawn advance extent of the line and its baseline.
    pub left: i64,
    pub right: i64,
    pub baseline: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DrawPage {
    pub layout_id: String,
    pub script_id: String,
    pub riwayah: String,
    pub font: String,
    pub page: u32,
    /// Page units (26.6).
    pub width: i64,
    pub height: i64,
    /// The page's font scale (16.16).
    pub scale: i64,
    pub canonical: bool,
    /// Outlines of the glyphs this page uses, by glyph id.
    pub glyphs: std::collections::BTreeMap<u32, Vec<PathOp>>,
    pub items: Vec<DrawItem>,
    pub lines: Vec<LineSummary>,
    pub notes: Vec<FitNote>,
    /// "Differs from Ḥafṣ" flags (T6.12, D37, D45), set only in highlight mode. They never
    /// change `items`, `lines` or anything drawn: plain and highlighted pages differ only here.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<Highlight>,
}

/// One highlighted cluster: what makes it differ from Ḥafṣ, and where that comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Highlight {
    pub line: u32,
    pub cluster: ClusterRef,
    /// The rule (or curated farsh entry) that changed this cluster.
    pub label: String,
    /// The cluster's Ḥafṣ form as `U+XXXX` codepoints; empty for an inserted cluster.
    pub hafs: String,
    /// Book and page, when cited.
    pub cite: Option<String>,
    /// False for an uncited hypothesis (shown in a distinct style, owner's choice at G4).
    pub cited: bool,
    /// True when the difference is pronunciation only (annotation channel, D40).
    pub annotation: bool,
}

/// Names that go into the page's metadata.
pub struct PageMeta<'a> {
    pub layout_id: &'a str,
    pub script_id: &'a str,
    pub riwayah: &'a str,
    pub font: &'a str,
}

/// `v * num / den`, rounded half up, exact (i128).
fn muldiv(v: i64, num: i64, den: i64) -> i64 {
    let (v, num, den) = (i128::from(v), i128::from(num), i128::from(den));
    let p = v * num;
    let q = (2 * p + den).div_euclid(2 * den);
    i64::try_from(q).expect("page coordinates fit in i64")
}

fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<PathOp> {
    let p = |v: i64| i32::try_from(v).expect("page coordinates fit in i32");
    vec![
        PathOp::Move { x: p(x0), y: p(y0) },
        PathOp::Line { x: p(x1), y: p(y0) },
        PathOp::Line { x: p(x1), y: p(y1) },
        PathOp::Line { x: p(x0), y: p(y1) },
        PathOp::Close,
    ]
}

/// The word EDITED as strokes on a 4 x 6 grid per letter (y down).
fn edited_label(center_x: i64, top: i64, height: i64) -> Vec<PathOp> {
    const E: &[&[(i64, i64)]] = &[&[(4, 0), (0, 0), (0, 6), (4, 6)], &[(0, 3), (3, 3)]];
    const D: &[&[(i64, i64)]] = &[&[(0, 0), (0, 6), (3, 6), (4, 5), (4, 1), (3, 0), (0, 0)]];
    const I: &[&[(i64, i64)]] = &[&[(2, 0), (2, 6)], &[(1, 0), (3, 0)], &[(1, 6), (3, 6)]];
    const T: &[&[(i64, i64)]] = &[&[(0, 0), (4, 0)], &[(2, 0), (2, 6)]];
    let letters = [E, D, I, T, E, D];
    let unit = height / 6;
    let advance = 6 * unit;
    let n = i64::try_from(letters.len()).expect("six letters");
    let left = center_x - (n * advance - 2 * unit) / 2;
    let p = |v: i64| i32::try_from(v).expect("page coordinates fit in i32");
    let mut out = Vec::new();
    for (i, letter) in letters.iter().enumerate() {
        let x0 = left + i64::try_from(i).expect("six letters") * advance;
        for stroke in *letter {
            for (k, (gx, gy)) in stroke.iter().enumerate() {
                let (x, y) = (p(x0 + gx * unit), p(top + gy * unit));
                out.push(if k == 0 {
                    PathOp::Move { x, y }
                } else {
                    PathOp::Line { x, y }
                });
            }
        }
    }
    out
}

/// Ink extent of a glyph outline placed at (`x`, `y`) (26.6 font units, y up), from its
/// on-curve and control points: (left, right, top).
fn ink(path: &[PathOp], x: i64, y: i64) -> Option<(i64, i64, i64)> {
    let mut out: Option<(i64, i64, i64)> = None;
    let mut add = |px: i32, py: i32| {
        let (px, py) = (x + i64::from(px), y + i64::from(py));
        out = Some(match out {
            None => (px, px, py),
            Some((l, r, t)) => (l.min(px), r.max(px), t.max(py)),
        });
    };
    for op in path {
        match *op {
            PathOp::Move { x, y } | PathOp::Line { x, y } => add(x, y),
            PathOp::Quad { x1, y1, x, y } => {
                add(x1, y1);
                add(x, y);
            }
            PathOp::Cubic {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                add(x1, y1);
                add(x2, y2);
                add(x, y);
            }
            PathOp::Close => {}
        }
    }
    out
}

fn place_line(
    l: &FitLine,
    scale: i64,
    top: i64,
    g: &Geometry,
    outlines: &Outlines,
    items: &mut Vec<DrawItem>,
    used: &mut BTreeSet<u32>,
) -> Result<LineSummary, Error> {
    let pitch = g.line_pitch * ONE;
    let baseline = top + pitch * BASELINE / 1000;
    // squeeze: positions scale by target/width on top of the page scale
    let (num, den) = l.squeeze.unwrap_or((1, 1));
    let sx = muldiv(scale, num, den);
    let drawn = muldiv(l.width, scale * num, SCALE_ONE * den);
    let block_left = g.margin_x * ONE;
    let block_right = (g.margin_x + g.line_width) * ONE;
    let right = if l.centered {
        (block_left + block_right + drawn + 1).div_euclid(2)
    } else {
        block_right
    };
    let left = right - drawn;
    let mut cursor = 0; // font units (26.6) from the line's right edge, reading order
    // ink (left, right, top) of each run of adjacent overlined words, font units
    let mut bars: Vec<(i64, i64, i64)> = Vec::new();
    let mut last_overlined = None;
    for (wi, w) in l.words.iter().enumerate() {
        let word_left = l.width - cursor - w.width; // from the line's left edge
        let mut word_ink: Option<(i64, i64, i64)> = None;
        let mut pen = 0;
        let mut gaps = w.gaps.clone();
        gaps.sort_by_key(|g| std::cmp::Reverse(g.0)); // visual order: later characters first
        let mut next_gap = 0;
        for gl in &w.glyphs {
            while next_gap < gaps.len() && i64::from(gl.cluster) <= gaps[next_gap].0 as i64 {
                pen += gaps[next_gap].1;
                next_gap += 1;
            }
            let x_fu = word_left + pen + i64::from(gl.x_offset) * ONE;
            let y_fu = i64::from(gl.y_offset) * ONE;
            let cluster = w
                .spans
                .iter()
                .find(|s| s.chars.contains(&(gl.cluster as usize)))
                .and_then(|s| s.clusters.first().cloned());
            items.push(DrawItem::Glyph {
                glyph: gl.glyph,
                x: left + muldiv(x_fu, sx, SCALE_ONE),
                y: baseline - muldiv(y_fu, scale, SCALE_ONE),
                sx,
                sy: scale,
                line: l.line,
                cluster,
            });
            used.insert(gl.glyph);
            if w.overline
                && let Some((gl_l, gl_r, gl_t)) = ink(outlines.get(gl.glyph)?, x_fu, y_fu)
            {
                word_ink = Some(match word_ink {
                    None => (gl_l, gl_r, gl_t),
                    Some((a, b, c)) => (a.min(gl_l), b.max(gl_r), c.max(gl_t)),
                });
            }
            pen += i64::from(gl.x_advance) * ONE;
        }
        if let Some((a, b, c)) = word_ink {
            // one bar over adjacent overlined words, as the 1441H print draws it
            match bars.last_mut() {
                Some(bar) if last_overlined.is_some_and(|i| i + 1 == wi) => {
                    *bar = (bar.0.min(a), bar.1.max(b), bar.2.max(c));
                }
                _ => bars.push((a, b, c)),
            }
            last_overlined = Some(wi);
        }
        cursor += w.width + l.spaces.get(wi).copied().unwrap_or(0);
    }
    // one height per line, above its tallest overlined word (as the 1441H print draws it)
    if let Some(ink_top) = bars.iter().map(|b| b.2).max() {
        let y0 = ink_top + g.overline_clearance * ONE;
        let y1 = y0 + g.overline_thickness * ONE;
        for (bl, br, _) in &bars {
            items.push(DrawItem::Fill {
                role: Role::SajdahOverline,
                line: l.line,
                path: rect(
                    left + muldiv(*bl, sx, SCALE_ONE),
                    baseline - muldiv(y1, scale, SCALE_ONE),
                    left + muldiv(*br, sx, SCALE_ONE),
                    baseline - muldiv(y0, scale, SCALE_ONE),
                ),
            });
        }
    }
    if matches!(l.kind, FitKind::SurahName { .. }) {
        items.push(DrawItem::Stroke {
            role: Role::Frame,
            width: 20 * ONE,
            path: rect(
                block_left,
                top + pitch * 8 / 100,
                block_right,
                top + pitch * 92 / 100,
            ),
        });
    }
    Ok(LineSummary {
        line: l.line,
        kind: l.kind.clone(),
        short_ppm: l.short_ppm,
        target: l.target,
        width: l.width,
        natural: l.natural,
        squeeze: l.squeeze,
        letter_gain: l.letter_gain,
        left,
        right,
        baseline,
    })
}

/// Draw commands for a fitted page. Deterministic and pure.
pub fn draw_page(
    fit: &FitPage,
    geometry: &Geometry,
    outlines: &Outlines,
    meta: &PageMeta,
) -> Result<DrawPage, Error> {
    let g = geometry;
    let width = (g.line_width + 2 * g.margin_x) * ONE;
    let height = (g.margin_top + LINES * g.line_pitch + g.margin_bottom) * ONE;
    let n = i64::try_from(fit.lines.len()).expect("few lines");
    let v_offset = g.line_pitch * ONE * (LINES - n).max(0) / 2;
    let mut items = Vec::new();
    let mut used = BTreeSet::new();
    let mut lines = Vec::new();
    for (i, l) in fit.lines.iter().enumerate() {
        let top =
            g.margin_top * ONE + v_offset + i64::try_from(i).expect("few") * g.line_pitch * ONE;
        lines.push(place_line(
            l, fit.scale, top, g, outlines, &mut items, &mut used,
        )?);
    }
    if !fit.canonical {
        let inset = 150 * ONE;
        items.push(DrawItem::Stroke {
            role: Role::EditedMarker,
            width: 60 * ONE,
            path: rect(inset, inset, width - inset, height - inset),
        });
        items.push(DrawItem::Stroke {
            role: Role::EditedMarker,
            width: 60 * ONE,
            path: edited_label(width / 2, 300 * ONE, 540 * ONE),
        });
    }
    Ok(DrawPage {
        layout_id: meta.layout_id.into(),
        script_id: meta.script_id.into(),
        riwayah: meta.riwayah.into(),
        font: meta.font.into(),
        page: fit.page,
        width,
        height,
        scale: fit.scale,
        canonical: fit.canonical,
        glyphs: outlines.subset(used)?,
        items,
        lines,
        notes: fit.notes.clone(),
        highlights: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muldiv_rounds_half_up_exactly() {
        assert_eq!(muldiv(3, 1, 2), 2);
        assert_eq!(muldiv(-3, 1, 2), -1);
        assert_eq!(muldiv(64, SCALE_ONE, SCALE_ONE), 64);
        assert_eq!(muldiv(i64::from(i32::MAX), 3, 3), i64::from(i32::MAX));
    }

    #[test]
    fn the_label_has_six_letters_of_strokes() {
        let moves = edited_label(0, 0, 600)
            .iter()
            .filter(|p| matches!(p, PathOp::Move { .. }))
            .count();
        assert_eq!(moves, 2 + 1 + 3 + 2 + 2 + 1);
    }
}
