//! Glyph outlines from the font's CFF2 or glyf table, exactly (D4, D10).
//!
//! CFF2: `read-fonts` evaluates each charstring to a sink with 16.16 fixed-point coordinates in
//! font units; they are rounded to 26.6 with integer arithmetic. glyf (TrueType, the KFGQPC
//! Warsh and Qālūn fonts, T5.3): contours become quadratic segments, with the implied on-curve
//! midpoint of two off-curve points exact in 26.6, and composite glyphs are flattened with their
//! F2Dot14 transforms in integer arithmetic. No floating point is involved, so the outlines are
//! the same bytes on every platform.

use std::collections::BTreeMap;

use mushaf_model::Error;
use read_fonts::ps::cff::CffFontRef;
use read_fonts::ps::cs::CommandSink;
use read_fonts::tables::glyf::{Anchor, CompositeGlyphFlags, Glyph};
use read_fonts::tables::loca::LocaGlyph;
use read_fonts::types::{F2Dot14, Fixed, GlyphId, Tag};
use read_fonts::{FontRef, TableProvider};
use serde::Serialize;

/// A path command in 26.6 font units, y up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum PathOp {
    Move {
        x: i32,
        y: i32,
    },
    Line {
        x: i32,
        y: i32,
    },
    /// A quadratic segment (TrueType outlines).
    Quad {
        x1: i32,
        y1: i32,
        x: i32,
        y: i32,
    },
    Cubic {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x: i32,
        y: i32,
    },
    Close,
}

/// 16.16 to 26.6, rounding half up.
fn to26(v: Fixed) -> i32 {
    (v.to_bits() + 512) >> 10
}

#[derive(Default)]
struct Sink(Vec<PathOp>);

impl CommandSink for Sink {
    fn move_to(&mut self, x: Fixed, y: Fixed) {
        self.0.push(PathOp::Move {
            x: to26(x),
            y: to26(y),
        });
    }
    fn line_to(&mut self, x: Fixed, y: Fixed) {
        self.0.push(PathOp::Line {
            x: to26(x),
            y: to26(y),
        });
    }
    fn curve_to(&mut self, cx0: Fixed, cy0: Fixed, cx1: Fixed, cy1: Fixed, x: Fixed, y: Fixed) {
        self.0.push(PathOp::Cubic {
            x1: to26(cx0),
            y1: to26(cy0),
            x2: to26(cx1),
            y2: to26(cy1),
            x: to26(x),
            y: to26(y),
        });
    }
    fn close(&mut self) {
        self.0.push(PathOp::Close);
    }
}

/// Every glyph's outline, extracted once.
pub struct Outlines {
    glyphs: Vec<Vec<PathOp>>,
}

impl Outlines {
    pub fn new(font: &[u8]) -> Result<Self, Error> {
        let err = |what: &str| Error::new(format!("outlines: {what}"));
        let font = FontRef::new(font).map_err(|_| err("cannot parse font"))?;
        let upem = i32::from(
            font.head()
                .map_err(|_| err("no head table"))?
                .units_per_em(),
        );
        if font.table_data(Tag::new(b"CFF2")).is_none() {
            return Self::from_glyf(&font);
        }
        let data = font
            .table_data(Tag::new(b"CFF2"))
            .ok_or_else(|| err("no CFF2 table"))?;
        let cff = CffFontRef::new_cff2(data.as_bytes(), Some(upem)).map_err(|_| err("bad CFF2"))?;
        if cff.matrix().is_some() || cff.var_store().is_some() {
            return Err(err("font matrix or variations are not supported"));
        }
        let mut glyphs = Vec::with_capacity(cff.num_glyphs() as usize);
        for gid in 0..cff.num_glyphs() {
            let gid = GlyphId::new(gid);
            let index = cff.subfont_index(gid).ok_or_else(|| err("no subfont"))?;
            let sub = cff.subfont(index, &[]).map_err(|_| err("bad subfont"))?;
            let mut sink = Sink::default();
            cff.evaluate_charstring(&sub, gid, &[], &mut sink)
                .map_err(|_| err("bad charstring"))?;
            glyphs.push(sink.0);
        }
        Ok(Outlines { glyphs })
    }

    /// TrueType outlines (T5.3).
    fn from_glyf(font: &FontRef) -> Result<Self, Error> {
        let err = |what: String| Error::new(format!("outlines: {what}"));
        let n = font
            .maxp()
            .map_err(|_| err("no maxp table".into()))?
            .num_glyphs();
        let glyf = font
            .glyf()
            .map_err(|_| err("neither a CFF2 nor a glyf table".into()))?;
        let loca = font.loca(None).map_err(|_| err("no loca table".into()))?;
        let mut glyphs = Vec::with_capacity(usize::from(n));
        for gid in 0..u32::from(n) {
            let mut ops = Vec::new();
            glyf_ops(&loca, &glyf, gid, 0, &mut ops).map_err(err)?;
            glyphs.push(ops);
        }
        Ok(Outlines { glyphs })
    }

    pub fn get(&self, glyph: u32) -> Result<&[PathOp], Error> {
        self.glyphs
            .get(glyph as usize)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::new(format!("no glyph {glyph}")))
    }

    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// The outlines of the given glyphs, by glyph id (for a page's definitions).
    pub fn subset(
        &self,
        glyphs: impl IntoIterator<Item = u32>,
    ) -> Result<BTreeMap<u32, Vec<PathOp>>, Error> {
        glyphs
            .into_iter()
            .map(|g| self.get(g).map(|p| (g, p.to_vec())))
            .collect()
    }
}

/// Nested composites deeper than this are rejected (fonts use one or two levels).
const MAX_DEPTH: u32 = 8;

fn glyf_ops(
    loca: &read_fonts::tables::loca::Loca,
    glyf: &read_fonts::tables::glyf::Glyf,
    gid: u32,
    depth: u32,
    out: &mut Vec<PathOp>,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err(format!("glyph {gid}: composites nest too deeply"));
    }
    let glyph = match loca.get(GlyphId::new(gid), glyf) {
        Some(LocaGlyph::Glyph(g)) => g,
        Some(LocaGlyph::Empty) => return Ok(()),
        _ => return Err(format!("glyph {gid}: unreadable")),
    };
    match glyph {
        Glyph::Simple(g) => {
            let points: Vec<(i32, i32, bool)> = g
                .points()
                .map(|p| (i32::from(p.x), i32::from(p.y), p.on_curve))
                .collect();
            let ends: Vec<usize> = g
                .end_pts_of_contours()
                .iter()
                .map(|e| usize::from(e.get()))
                .collect();
            if ends.last().is_some_and(|&e| e >= points.len()) {
                return Err(format!("glyph {gid}: contour ends past its points"));
            }
            simple_ops(&points, &ends, out);
        }
        Glyph::Composite(g) => {
            for c in g.components() {
                if c.flags
                    .contains(CompositeGlyphFlags::SCALED_COMPONENT_OFFSET)
                {
                    return Err(format!("glyph {gid}: scaled component offsets"));
                }
                let Anchor::Offset { x: dx, y: dy } = c.anchor else {
                    return Err(format!("glyph {gid}: point-matched components"));
                };
                let mut child = Vec::new();
                glyf_ops(
                    loca,
                    glyf,
                    u32::from(c.glyph.to_u16()),
                    depth + 1,
                    &mut child,
                )?;
                let t = c.transform;
                let (dx, dy) = (i32::from(dx) * 64, i32::from(dy) * 64);
                let map = |x: i32, y: i32| {
                    let (x2, y2) = transform(t.xx, t.xy, t.yx, t.yy, x, y);
                    (x2 + dx, y2 + dy)
                };
                out.extend(child.into_iter().map(|op| map_op(op, &map)));
            }
        }
    }
    Ok(())
}

/// `(xx * x + xy * y, yx * x + yy * y)` with F2Dot14 factors, rounded half up, in integers.
fn transform(xx: F2Dot14, xy: F2Dot14, yx: F2Dot14, yy: F2Dot14, x: i32, y: i32) -> (i32, i32) {
    let f = |a: F2Dot14, b: F2Dot14| {
        let v = i64::from(a.to_bits()) * i64::from(x) + i64::from(b.to_bits()) * i64::from(y);
        ((v + (1 << 13)) >> 14) as i32
    };
    (f(xx, xy), f(yx, yy))
}

fn map_op(op: PathOp, map: &impl Fn(i32, i32) -> (i32, i32)) -> PathOp {
    match op {
        PathOp::Move { x, y } => {
            let (x, y) = map(x, y);
            PathOp::Move { x, y }
        }
        PathOp::Line { x, y } => {
            let (x, y) = map(x, y);
            PathOp::Line { x, y }
        }
        PathOp::Quad { x1, y1, x, y } => {
            let ((x1, y1), (x, y)) = (map(x1, y1), map(x, y));
            PathOp::Quad { x1, y1, x, y }
        }
        PathOp::Cubic {
            x1,
            y1,
            x2,
            y2,
            x,
            y,
        } => {
            let ((x1, y1), (x2, y2), (x, y)) = (map(x1, y1), map(x2, y2), map(x, y));
            PathOp::Cubic {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            }
        }
        PathOp::Close => PathOp::Close,
    }
}

/// The contours of a simple glyph as path ops in 26.6. A contour starts at its first on-curve
/// point (or, with none, at the midpoint of its last and first points); two off-curve points in
/// a row imply the on-curve point midway, which is exact in 26.6 (`32 * (a + b)`). The closing
/// segment back to the start is implied by `Close` when it is a straight line.
fn simple_ops(points: &[(i32, i32, bool)], ends: &[usize], out: &mut Vec<PathOp>) {
    let mut start = 0;
    for &end in ends {
        if end < start {
            continue;
        }
        let c: Vec<(i32, i32, bool)> = points[start..=end]
            .iter()
            .map(|&(x, y, on)| (x * 64, y * 64, on))
            .collect();
        start = end + 1;
        let n = c.len();
        let (s, order): ((i32, i32), Vec<usize>) = match c.iter().position(|p| p.2) {
            Some(i) => ((c[i].0, c[i].1), (1..=n).map(|k| (i + k) % n).collect()),
            None => (
                ((c[n - 1].0 + c[0].0) / 2, (c[n - 1].1 + c[0].1) / 2),
                (0..n).collect(),
            ),
        };
        out.push(PathOp::Move { x: s.0, y: s.1 });
        let mut ctrl: Option<(i32, i32)> = None;
        for (k, &i) in order.iter().enumerate() {
            let (x, y, on) = c[i];
            match (on, ctrl) {
                (true, None) => {
                    if !(k == order.len() - 1 && (x, y) == s) {
                        out.push(PathOp::Line { x, y });
                    }
                }
                (true, Some((x1, y1))) => {
                    out.push(PathOp::Quad { x1, y1, x, y });
                    ctrl = None;
                }
                (false, None) => ctrl = Some((x, y)),
                (false, Some((x1, y1))) => {
                    let (mx, my) = ((x1 + x) / 2, (y1 + y) / 2);
                    out.push(PathOp::Quad {
                        x1,
                        y1,
                        x: mx,
                        y: my,
                    });
                    ctrl = Some((x, y));
                }
            }
        }
        if let Some((x1, y1)) = ctrl {
            out.push(PathOp::Quad {
                x1,
                y1,
                x: s.0,
                y: s.1,
            });
        }
        out.push(PathOp::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_to_26_6_rounds_half_up() {
        assert_eq!(to26(Fixed::from_bits(1 << 16)), 64);
        assert_eq!(to26(Fixed::from_bits(512)), 1); // 1/128 unit rounds up to 1/64
        assert_eq!(to26(Fixed::from_bits(511)), 0);
        assert_eq!(to26(Fixed::from_bits(-512)), 0);
        assert_eq!(to26(Fixed::from_bits(-513)), -1);
    }

    #[test]
    fn implied_midpoints_and_off_curve_only_contours() {
        let mut out = Vec::new();
        // on, off, off, on -> line-free: quad to the implied midpoint, then quad to the end
        simple_ops(
            &[(0, 0, true), (2, 4, false), (6, 4, false), (8, 0, true)],
            &[3],
            &mut out,
        );
        assert_eq!(
            out,
            [
                PathOp::Move { x: 0, y: 0 },
                PathOp::Quad {
                    x1: 128,
                    y1: 256,
                    x: 256,
                    y: 256
                },
                PathOp::Quad {
                    x1: 384,
                    y1: 256,
                    x: 512,
                    y: 0
                },
                PathOp::Close,
            ]
        );
        out.clear();
        simple_ops(&[(0, 2, false), (2, 0, false)], &[1], &mut out);
        assert_eq!(out[0], PathOp::Move { x: 64, y: 64 });
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn transforms_round_half_up() {
        let one = F2Dot14::from_bits(1 << 14);
        let half = F2Dot14::from_bits(1 << 13);
        let zero = F2Dot14::from_bits(0);
        assert_eq!(transform(one, zero, zero, one, 5, -7), (5, -7));
        assert_eq!(transform(half, zero, zero, half, 3, -3), (2, -1));
    }
}
