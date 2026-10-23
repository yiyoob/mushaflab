//! Native layouts per riwāyah (T5.2, D1, D29): the riwāyah's own KFGQPC page and line data,
//! given per ayah, as hard constraints on its resolved text; inside them, every line break that
//! the data leaves open takes the 1441H Ḥafṣ break of the same page and line when that is
//! consistent, and is otherwise fitted by width. Every line says which of the three it is.
//!
//! One dynamic programme over all text lines in reading order chooses each line's first item.
//! A line may hold an item only within its ayah's lines; an ayah's first item must open its first
//! line and its last item close its last line. Among the solutions, the one with the fewest lines
//! that do not start at the Ḥafṣ break wins; ties are broken by the sum of squared differences
//! between each line's natural width and a target width, then by the earliest start. All
//! arithmetic is integer, so the result is deterministic (D10).

use std::collections::BTreeMap;

use mushaf_model::{Error, FitTag, ItemRef, Layout, Line, LineKind, Spine, SurahHeader};
use mushaf_resolve::{Origin, Resolved};
use serde::{Deserialize, Serialize};

/// One ayah of `data/layout_inputs/<riwayah>.ayah-lines.yaml`, as KFGQPC gives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AyahLines {
    pub surah: u32,
    pub ayah: u32,
    pub page: u32,
    /// `page + 1` for an ayah that crosses a page.
    pub last_page: u32,
    pub line_start: u32,
    pub line_end: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AyahLinesFile {
    pub riwayah: String,
    pub source: String,
    pub spine: String,
    pub ayahs: Vec<AyahLines>,
}

/// The fitting inputs for one riwāyah.
pub struct NativeInput<'a> {
    pub riwayah: &'a str,
    pub layout_id: &'a str,
    pub text_source: &'a str,
    /// The riwāyah's resolved text (data mode).
    pub base: &'a Resolved,
    /// Each ayah's first and last item in `base`, with its page and lines, in reading order.
    pub ayahs: Vec<(usize, usize, AyahLines)>,
    /// The 1441H Ḥafṣ layout and its spine: the breaks to prefer.
    pub hafs: &'a Layout,
    pub spine: &'a Spine,
    /// Number of lines on each page (pages 1 and 2 are shorter).
    pub lines_per_page: BTreeMap<u32, u32>,
    pub headers: &'a [SurahHeader],
    /// Natural width of each `base` item in 26.6, `None` where it cannot be measured (an item
    /// whose clusters have no encoding row yet, G-102).
    pub widths: Vec<Option<i64>>,
    /// Width of one space (26.6).
    pub space: i64,
}

/// What the fitting found about one text line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LineReport {
    pub page: u32,
    pub line: u32,
    pub fit: FitTag,
    pub first: String,
    pub last: String,
    /// Natural width of the line (26.6); `None` if an item could not be measured.
    pub natural: Option<i64>,
    /// Lines that start where the Ḥafṣ line of the same page and line starts.
    pub hafs_start: Option<String>,
    /// For a fitted line: how many first items the constraints allow, and the smallest cost
    /// increase (in font units, the square root of the squared-width difference) of moving the
    /// break one item either way.
    pub alternatives: Option<(usize, Option<i64>)>,
}

#[derive(Clone, Debug)]
pub struct NativeLayout {
    pub layout: Layout,
    pub report: Vec<LineReport>,
    /// The width the fitting aimed at (26.6): the median natural width of lines whose breaks
    /// the data fix on both sides.
    pub target: i64,
}

type Pos = (u32, u32);

/// Per base item: the range of text-line indices its ayah allows, and whether it must open or
/// close that range.
struct ItemRule {
    lo: usize,
    hi: usize,
    first: bool,
    last: bool,
}

fn err(msg: impl Into<String>) -> Error {
    Error::new(msg.into())
}

/// Text lines in reading order: every page and line some ayah covers.
fn text_lines(input: &NativeInput) -> Result<Vec<Pos>, Error> {
    let per = |p: u32| {
        input
            .lines_per_page
            .get(&p)
            .copied()
            .ok_or_else(|| err(format!("page {p} has no line count")))
    };
    let mut set = std::collections::BTreeSet::new();
    for (_, _, a) in &input.ayahs {
        if a.page == a.last_page {
            for l in a.line_start..=a.line_end {
                set.insert((a.page, l));
            }
        } else {
            if a.last_page != a.page + 1 {
                return Err(err(format!("{}:{}: bad page range", a.surah, a.ayah)));
            }
            for l in a.line_start..=per(a.page)? {
                set.insert((a.page, l));
            }
            for l in 1..=a.line_end {
                set.insert((a.last_page, l));
            }
        }
    }
    for &(p, l) in &set {
        if l == 0 || l > per(p)? {
            return Err(err(format!("line {p}:{l} is outside its page")));
        }
    }
    Ok(set.into_iter().collect())
}

fn item_rules(input: &NativeInput, index: &BTreeMap<Pos, usize>) -> Result<Vec<ItemRule>, Error> {
    let n = input.base.items.len();
    let mut rules: Vec<Option<ItemRule>> = (0..n).map(|_| None).collect();
    let mut next = 0;
    for (first, last, a) in &input.ayahs {
        if *first != next || last < first {
            return Err(err(format!(
                "ayah {}:{} does not follow the previous one in the resolved text",
                a.surah, a.ayah
            )));
        }
        let lo = index[&(a.page, a.line_start)];
        let hi = index[&(a.last_page, a.line_end)];
        if hi < lo {
            return Err(err(format!(
                "ayah {}:{} ends before it starts",
                a.surah, a.ayah
            )));
        }
        for (k, rule) in rules.iter_mut().enumerate().take(last + 1).skip(*first) {
            *rule = Some(ItemRule {
                lo,
                hi,
                first: k == *first,
                last: k == *last,
            });
        }
        next = last + 1;
    }
    if next != n {
        return Err(err("the ayahs do not cover the resolved text"));
    }
    Ok(rules.into_iter().map(|r| r.expect("covered")).collect())
}

/// For each text line, the base index where the Ḥafṣ line at the same page and line starts: the
/// first item whose spine anchor is at or after that line's first spine item.
fn hafs_starts(input: &NativeInput, lines: &[Pos]) -> Result<Vec<Option<usize>>, Error> {
    let mut first_of: BTreeMap<Pos, &ItemRef> = BTreeMap::new();
    for l in &input.hafs.lines {
        if let LineKind::Text { first, .. } = &l.kind {
            first_of.insert((l.page, l.line), first);
        }
    }
    let anchors: Vec<usize> = input
        .base
        .items
        .iter()
        .map(|it| match it.origin {
            Origin::Spine { pos } | Origin::Inserted { anchor: pos } => pos,
        })
        .collect();
    lines
        .iter()
        .map(|pos| {
            first_of
                .get(pos)
                .map(|id| {
                    let sp = input
                        .spine
                        .position(id)
                        .ok_or_else(|| err(format!("Hafs layout names unknown item {id}")))?;
                    Ok(anchors.partition_point(|&a| a < sp))
                })
                .transpose()
        })
        .collect()
}

/// Cost of a solution: lines not at the Ḥafṣ break, then squared width differences.
type Cost = (u32, i128);

fn line_width(input: &NativeInput, p: usize, e: usize) -> Option<i64> {
    let mut w = 0;
    for k in p..e {
        w += input.widths[k]?;
    }
    Some(w + input.space * i64::try_from(e - p - 1).expect("few items"))
}

fn sq(w: Option<i64>, target: i64) -> i128 {
    w.map_or(0, |w| {
        let d = i128::from(w - target);
        d * d
    })
}

fn allowed(r: &ItemRule, j: usize) -> bool {
    r.lo <= j && j <= r.hi && !(r.first && j != r.lo) && !(r.last && j != r.hi)
}

/// Fits the native layout of one riwāyah.
pub fn fit_native(input: &NativeInput) -> Result<NativeLayout, Error> {
    let lines = text_lines(input)?;
    let index: BTreeMap<Pos, usize> = lines.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    let rules = item_rules(input, &index)?;
    let hafs = hafs_starts(input, &lines)?;
    let n = input.base.items.len();
    let m = lines.len();
    // a line start is forced where an ayah opens on that line or one closed on the line before
    let forced = |j: usize, p: usize| {
        j == 0
            || (rules[p].first && rules[p].lo == j)
            || (p > 0 && rules[p - 1].last && rules[p - 1].hi + 1 == j)
    };
    // target: median natural width of lines that hold whole ayahs only, all of which start
    // and end on that line (both breaks fixed by the data)
    let mut touching: Vec<Vec<(usize, usize, bool)>> = vec![Vec::new(); m];
    for (first, last, _) in &input.ayahs {
        let (lo, hi) = (rules[*first].lo, rules[*first].hi);
        for t in touching.iter_mut().take(hi + 1).skip(lo) {
            t.push((*first, *last, lo == hi));
        }
    }
    let mut fixed: Vec<i64> = touching
        .iter()
        .filter(|t| !t.is_empty() && t.iter().all(|a| a.2))
        .filter_map(|t| line_width(input, t[0].0, t[t.len() - 1].1 + 1))
        .collect();
    fixed.sort_unstable();
    let target = *fixed
        .get(fixed.len() / 2)
        .ok_or_else(|| err("no line has both breaks fixed by the data"))?;
    // forward DP: best[j][p] = cheapest way to lay lines ..j with line j starting at item p
    let mut best: Vec<BTreeMap<usize, (Cost, usize)>> = vec![BTreeMap::new(); m + 1];
    best[0].insert(0, ((0, 0), usize::MAX));
    for j in 0..m {
        let states: Vec<(usize, Cost)> = best[j].iter().map(|(&p, &(c, _))| (p, c)).collect();
        for (p, cost) in states {
            if p >= n || !allowed(&rules[p], j) {
                continue;
            }
            let miss = u32::from(hafs[j].is_some_and(|h| h != p) && !forced(j, p));
            let mut e = p + 1;
            loop {
                // the line holds [p, e); item e, if any, must be allowed to open line j + 1,
                // which the next round checks
                if (e == n) == (j + 1 == m) {
                    let c = (cost.0 + miss, cost.1 + sq(line_width(input, p, e), target));
                    let slot = best[j + 1].entry(e).or_insert((c, p));
                    if c < slot.0 {
                        *slot = (c, p);
                    }
                }
                if e == n || !allowed(&rules[e], j) {
                    break;
                }
                e += 1;
            }
        }
    }
    if !best[m].contains_key(&n) {
        return Err(err(format!(
            "{}: no line breaks satisfy the ayah lines (C safety net would be needed)",
            input.riwayah
        )));
    }
    // backtrack
    let mut starts = vec![0usize; m];
    let mut e = n;
    for j in (0..m).rev() {
        let p = best[j + 1][&e].1;
        starts[j] = p;
        e = p;
    }
    let ends: Vec<usize> = (0..m)
        .map(|j| if j + 1 < m { starts[j + 1] } else { n })
        .collect();
    let mut report = Vec::with_capacity(m);
    let mut text = BTreeMap::new();
    for j in 0..m {
        let (p, e) = (starts[j], ends[j]);
        let fit = if forced(j, p) {
            FitTag::Forced
        } else if hafs[j] == Some(p) {
            FitTag::Hafs
        } else {
            FitTag::Fitted
        };
        let alternatives = (fit == FitTag::Fitted).then(|| {
            let prev = starts[j - 1];
            let next = ends[j];
            let here =
                sq(line_width(input, prev, p), target) + sq(line_width(input, p, next), target);
            let mut options = 0;
            let mut margin: Option<i128> = None;
            for q in prev + 1..next {
                let ok = (prev..q).all(|k| allowed(&rules[k], j - 1))
                    && (q..next).all(|k| allowed(&rules[k], j));
                if !ok {
                    continue;
                }
                options += 1;
                if q + 1 == p || q == p + 1 {
                    let c = sq(line_width(input, prev, q), target)
                        + sq(line_width(input, q, next), target);
                    let d = c - here;
                    margin = Some(margin.map_or(d, |x| x.min(d)));
                }
            }
            (options, margin.map(|d| isqrt(d.max(0))))
        });
        let (page, line) = lines[j];
        report.push(LineReport {
            page,
            line,
            fit,
            first: input.base.items[p].id.to_string(),
            last: input.base.items[e - 1].id.to_string(),
            natural: line_width(input, p, e),
            hafs_start: hafs[j].map(|h| input.base.items[h].id.to_string()),
            alternatives,
        });
        text.insert(
            (page, line),
            Line {
                page,
                line,
                kind: LineKind::Text {
                    first: input.base.items[p].id.clone(),
                    last: input.base.items[e - 1].id.clone(),
                },
                centered: false,
                correction: None,
                fit: Some(fit),
            },
        );
    }
    let mut all = text;
    for h in input.headers {
        let mut add = |line: u32, kind: LineKind| {
            if all.contains_key(&(h.page, line)) {
                return Err(err(format!("header line {}:{line} holds text", h.page)));
            }
            all.insert(
                (h.page, line),
                Line {
                    page: h.page,
                    line,
                    kind,
                    centered: true,
                    correction: None,
                    fit: None,
                },
            );
            Ok(())
        };
        add(h.name_line, LineKind::SurahName { surah: h.surah })?;
        if let Some(b) = h.basmalah_line {
            add(b, LineKind::Basmalah)?;
        }
    }
    let pages = input.lines_per_page.keys().copied().max().unwrap_or(0);
    let layout = Layout {
        layout_id: input.layout_id.into(),
        script_id: input.hafs.script_id.clone(),
        spine: input.spine.id.clone(),
        riwayah: Some(input.riwayah.into()),
        pages,
        lines_per_page: input.hafs.lines_per_page,
        sources: [
            ("ayah_lines".to_string(), input.text_source.to_string()),
            ("breaks".to_string(), input.hafs.layout_id.clone()),
        ]
        .into_iter()
        .collect(),
        lines: all.into_values().collect(),
    };
    layout.validate()?;
    Ok(NativeLayout {
        layout,
        report,
        target,
    })
}

/// Integer square root (floor).
fn isqrt(v: i128) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut x = v;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    i64::try_from(x).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_square_root() {
        for v in [0i128, 1, 2, 3, 4, 15, 16, 17, 1 << 40, (1 << 40) + 1] {
            let r = i128::from(isqrt(v));
            assert!(r * r <= v && (r + 1) * (r + 1) > v, "{v}");
        }
    }
}

/// Each ayah of `lines` (in the riwāyah's own numbering) as its first and last item in `base`:
/// through the riwāyah's ayah map, or, for a text numbered like the spine (Ḥafṣ), its spine ids.
pub fn ayah_ranges(
    base: &Resolved,
    map: Option<&mushaf_model::AyahMap>,
    lines: &AyahLinesFile,
) -> Result<Vec<(usize, usize, AyahLines)>, Error> {
    let pos: BTreeMap<&ItemRef, usize> = base
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| (&it.id, i))
        .collect();
    let mut bounds: Vec<((u32, u32), usize, usize)> = Vec::new();
    match map {
        Some(m) => {
            for e in &m.entries {
                let find = |r: &ItemRef| {
                    pos.get(r)
                        .copied()
                        .ok_or_else(|| err(format!("ayah map item {r} is not in the text")))
                };
                bounds.push(((e.surah, e.ayah), find(&e.first)?, find(&e.last)?));
            }
        }
        None => {
            for (i, it) in base.items.iter().enumerate() {
                if !it.id.is_spine() {
                    return Err(err(format!(
                        "{} is inserted but the text has no ayah map",
                        it.id
                    )));
                }
                let key = (it.id.surah, it.id.ayah);
                match bounds.last_mut() {
                    Some((k, _, last)) if *k == key => *last = i,
                    _ => bounds.push((key, i, i)),
                }
            }
        }
    }
    if bounds.len() != lines.ayahs.len() {
        return Err(err(format!(
            "{} ayahs in the text, {} in the layout data",
            bounds.len(),
            lines.ayahs.len()
        )));
    }
    bounds
        .into_iter()
        .zip(&lines.ayahs)
        .map(|((key, a, b), l)| {
            if key != (l.surah, l.ayah) {
                return Err(err(format!(
                    "ayah {}:{} of the text meets {}:{} of the layout data",
                    key.0, key.1, l.surah, l.ayah
                )));
            }
            Ok((a, b, l.clone()))
        })
        .collect()
}

/// Lines on each page of a layout (the highest line number used).
pub fn lines_per_page(layout: &Layout) -> BTreeMap<u32, u32> {
    let mut out = BTreeMap::new();
    for l in &layout.lines {
        let e = out.entry(l.page).or_insert(0);
        *e = (*e).max(l.line);
    }
    out
}

/// How a native line's width was decided (P5, D30).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortBasis {
    /// The Ḥafṣ line at the same page and line holds the same items and is short in the print.
    Hafs,
    /// The Ḥafṣ line at the same page and line is short in the print but holds other items: the
    /// line is short at its own natural share of a full line.
    HafsPosition,
    /// Its natural width is under `NATURAL_SHORT_PPM` of the median full line.
    Width,
}

/// Short lines by (page, line): width in ppm of a full line, and the basis.
pub type ShortLines = BTreeMap<(u32, u32), (i64, ShortBasis)>;

/// A line whose *natural* width is under this share of the median full line is short. The 90%
/// rule of `line_widths.py` applies to the print's own (justified) widths; natural widths spread
/// much more, because the print stretches letters. Calibrated on Ḥafṣ (KFGQPC Ḥafṣ font, the
/// print's 30 short lines known): at 90% 1,486 of 8,790 full lines would count as short, at 60%
/// none does (the narrowest full line is at 63%), and 14 of the 30 short lines are found (D30).
pub const NATURAL_SHORT_PPM: i64 = 600_000;

/// Short lines of a native layout, with their width in ppm of a full line and the basis.
/// `hafs_short` is the Ḥafṣ print's list (`LineWidths::short`).
pub fn short_lines(
    input: &NativeInput,
    fitted: &NativeLayout,
    hafs_short: &BTreeMap<(u32, u32), i64>,
) -> Result<ShortLines, Error> {
    let mut hafs_items: BTreeMap<Pos, (&ItemRef, &ItemRef)> = BTreeMap::new();
    for l in &input.hafs.lines {
        if let LineKind::Text { first, last } = &l.kind {
            hafs_items.insert((l.page, l.line), (first, last));
        }
    }
    let same = |r: &LineReport| {
        hafs_items
            .get(&(r.page, r.line))
            .is_some_and(|(f, l)| f.to_string() == r.first && l.to_string() == r.last)
    };
    let mut full: Vec<i64> = fitted
        .report
        .iter()
        .filter(|r| same(r) && !hafs_short.contains_key(&(r.page, r.line)))
        .filter_map(|r| r.natural)
        .collect();
    full.sort_unstable();
    let median = *full
        .get(full.len() / 2)
        .ok_or_else(|| err("no full line to measure against"))?;
    let mut out = BTreeMap::new();
    for r in &fitted.report {
        let key = (r.page, r.line);
        if same(r) {
            if let Some(&ppm) = hafs_short.get(&key) {
                out.insert(key, (ppm, ShortBasis::Hafs));
            }
        } else if hafs_short.contains_key(&key)
            && let Some(w) = r.natural
        {
            out.insert(
                key,
                (
                    (w * 1_000_000 / median).min(1_000_000),
                    ShortBasis::HafsPosition,
                ),
            );
        } else if let Some(w) = r.natural {
            let ppm = w * 1_000_000 / median;
            if ppm < NATURAL_SHORT_PPM {
                out.insert(key, (ppm, ShortBasis::Width));
            }
        }
    }
    Ok(out)
}
