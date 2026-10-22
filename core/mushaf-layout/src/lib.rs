//! Line mapping (T2.6) and, later, overflow policy (T5.2).
//!
//! A layout gives each text line its first and last spine item. A resolved item belongs to the
//! line holding its spine anchor: a spine item's own position, or for an inserted item the
//! nearest spine node before it in reading order (deleted or not), or the first text line if
//! there is none. A deleted anchor contributes nothing, so its line starts (or ends) at the
//! next surviving item. Because anchors never decrease in reading order, each line holds a
//! contiguous run of resolved items, and the runs cover all of them in order.

use std::fmt;
use std::ops::Range;

use mushaf_model::{Layout, LineKind, Spine};
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
    // the text line (index into `starts`) of each resolved item
    let mut text_line = Vec::with_capacity(resolved.items.len());
    for it in &resolved.items {
        let anchor = match it.origin {
            Origin::Spine { pos } => Some(pos),
            Origin::Inserted { after_spine } => after_spine,
        };
        let t = anchor.map_or(0, |p| starts.partition_point(|&(_, s)| s <= p) - 1);
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
