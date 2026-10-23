//! Line mapping (T2.6) and page fitting with the overflow policy (`fit`, T3.2, D15).
//!
//! A layout gives each text line its first and last spine item. A resolved item belongs to the
//! line holding its spine anchor: a spine item's own position, or for an inserted item the
//! spine item its insert chain starts from, deleted or not. So an insert-before joins the line
//! of the word it precedes and an insert-after the line of the word it follows, and nested
//! inserts follow their own anchor (owner decision at the Phase 2 gate, 2026-10-22). A deleted
//! anchor contributes nothing, so its line starts (or ends) at the next surviving item.
//! Because anchors never decrease in reading order, each line holds a contiguous run of
//! resolved items, and the runs cover all of them in order.
//!
//! A riwāyah's native layout (T5.2, D29) names items of that riwāyah's resolved text, inserted
//! ones included (`map_lines_on`): an item belongs to the line of its own id in that text, or,
//! for an item an edit overlay inserted, of the nearest id its insert chain starts from.

pub mod fit;
pub mod native;

use std::fmt;
use std::ops::Range;

use std::collections::BTreeMap;

use mushaf_model::{ItemRef, Layout, LineKind, Spine};
use mushaf_resolve::{Origin, Resolved};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MappedLine {
    pub page: u32,
    pub line: u32,
    pub kind: LineKind,
    pub centered: bool,
    /// Indices into `Resolved::items`; empty for surah-name and basmalah lines.
    pub items: Range<usize>,
}

/// Advisory findings about the mapping.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LineNote {
    /// A text line left with no clusters (all its anchors deleted, nothing inserted).
    EmptyLine { page: u32, line: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mapping {
    pub lines: Vec<MappedLine>,
    pub report: Vec<LineNote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutError {
    SpineMismatch {
        layout: String,
        spine: String,
    },
    /// A line anchor is not a spine item.
    UnknownAnchor {
        page: u32,
        line: u32,
        id: String,
    },
    /// Text lines must cover the spine's items once each, contiguously and in order.
    NotContiguous {
        page: u32,
        line: u32,
    },
    NotCovered,
    /// Resolved anchors went backwards (a resolver invariant is broken).
    OutOfOrder {
        item: String,
    },
    /// A native layout used for another riwāyah's text, or an item whose insert chain reaches
    /// no item of the riwāyah's text.
    NotInStream {
        item: String,
    },
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::SpineMismatch { layout, spine } => {
                write!(f, "layout for spine {layout:?} used with spine {spine:?}")
            }
            LayoutError::UnknownAnchor { page, line, id } => {
                write!(f, "line {page}:{line}: anchor {id} is not a spine item")
            }
            LayoutError::NotContiguous { page, line } => {
                write!(
                    f,
                    "line {page}:{line} does not start right after the previous text line"
                )
            }
            LayoutError::NotCovered => f.write_str("text lines do not cover every spine item"),
            LayoutError::OutOfOrder { item } => write!(f, "resolved item {item} is out of order"),
            LayoutError::NotInStream { item } => {
                write!(f, "item {item} has no anchor in the layout's riwayah text")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// Spine position of each text line's first item, checked to tile the spine exactly.
fn line_starts(layout: &Layout, spine: &Spine) -> Result<Vec<(usize, usize)>, LayoutError> {
    if layout.spine != spine.id {
        return Err(LayoutError::SpineMismatch {
            layout: layout.spine.clone(),
            spine: spine.id.clone(),
        });
    }
    let mut starts = Vec::new(); // (line index in layout, first spine position)
    let mut next = 0;
    for (i, l) in layout.lines.iter().enumerate() {
        let LineKind::Text { first, last } = &l.kind else {
            continue;
        };
        let pos = |r: &mushaf_model::ItemRef| {
            spine.position(r).ok_or_else(|| LayoutError::UnknownAnchor {
                page: l.page,
                line: l.line,
                id: r.to_string(),
            })
        };
        let (a, b) = (pos(first)?, pos(last)?);
        if a != next || b < a {
            return Err(LayoutError::NotContiguous {
                page: l.page,
                line: l.line,
            });
        }
        starts.push((i, a));
        next = b + 1;
    }
    if next != spine.items.len() || starts.is_empty() {
        return Err(LayoutError::NotCovered);
    }
    Ok(starts)
}

pub fn map_lines(
    layout: &Layout,
    spine: &Spine,
    resolved: &Resolved,
) -> Result<Mapping, LayoutError> {
    let starts = line_starts(layout, spine)?;
    assign(layout, &starts, resolved, |it| {
        let (Origin::Spine { pos: anchor } | Origin::Inserted { anchor }) = it.origin;
        Ok(anchor)
    })
}

/// Line mapping on a native layout (T5.2): `base` is the riwāyah's resolved text the layout
/// names (data mode), `resolved` the same text with any edit overlay.
pub fn map_lines_on(
    layout: &Layout,
    base: &Resolved,
    resolved: &Resolved,
) -> Result<Mapping, LayoutError> {
    let pos: BTreeMap<&ItemRef, usize> = base
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| (&it.id, i))
        .collect();
    let mut starts = Vec::new();
    let mut next = 0;
    for (i, l) in layout.lines.iter().enumerate() {
        let LineKind::Text { first, last } = &l.kind else {
            continue;
        };
        let find = |r: &ItemRef| {
            pos.get(r)
                .copied()
                .ok_or_else(|| LayoutError::UnknownAnchor {
                    page: l.page,
                    line: l.line,
                    id: r.to_string(),
                })
        };
        let (a, b) = (find(first)?, find(last)?);
        if a != next || b < a {
            return Err(LayoutError::NotContiguous {
                page: l.page,
                line: l.line,
            });
        }
        starts.push((i, a));
        next = b + 1;
    }
    if next != base.items.len() || starts.is_empty() {
        return Err(LayoutError::NotCovered);
    }
    assign(layout, &starts, resolved, |it| {
        let mut id = it.id.clone();
        loop {
            if let Some(&p) = pos.get(&id) {
                return Ok(p);
            }
            if id.ins.pop().is_none() {
                return Err(LayoutError::NotInStream {
                    item: it.id.to_string(),
                });
            }
        }
    })
}

/// Assigns each resolved item to the text line holding its anchor (a position in the stream
/// the layout's line starts index).
fn assign(
    layout: &Layout,
    starts: &[(usize, usize)],
    resolved: &Resolved,
    anchor_of: impl Fn(&mushaf_resolve::RItem) -> Result<usize, LayoutError>,
) -> Result<Mapping, LayoutError> {
    // the text line (index into `starts`) of each resolved item
    let mut text_line = Vec::with_capacity(resolved.items.len());
    for it in &resolved.items {
        let anchor = anchor_of(it)?;
        let t = starts.partition_point(|&(_, s)| s <= anchor) - 1;
        if text_line.last().is_some_and(|&prev| prev > t) {
            return Err(LayoutError::OutOfOrder {
                item: it.id.to_string(),
            });
        }
        text_line.push(t);
    }
    let mut lines = Vec::with_capacity(layout.lines.len());
    let mut report = Vec::new();
    let (mut cursor, mut t) = (0, 0);
    for (i, l) in layout.lines.iter().enumerate() {
        let mut range = cursor..cursor;
        if t < starts.len() && starts[t].0 == i {
            let end = cursor + text_line[cursor..].iter().take_while(|&&x| x == t).count();
            range = cursor..end;
            if resolved.items[range.clone()]
                .iter()
                .all(|it| it.clusters.is_empty())
            {
                report.push(LineNote::EmptyLine {
                    page: l.page,
                    line: l.line,
                });
            }
            cursor = end;
            t += 1;
        }
        lines.push(MappedLine {
            page: l.page,
            line: l.line,
            kind: l.kind.clone(),
            centered: l.centered,
            items: range,
        });
    }
    debug_assert_eq!(cursor, resolved.items.len());
    Ok(Mapping { lines, report })
}
