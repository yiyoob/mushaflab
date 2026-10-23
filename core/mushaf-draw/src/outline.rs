//! Glyph outlines from the font's CFF2 table, exactly (D4, D10).
//!
//! `read-fonts` evaluates each charstring to a sink with 16.16 fixed-point coordinates in font
//! units; they are rounded to 26.6 with integer arithmetic. No floating point is involved, so
//! the outlines are the same bytes on every platform.

use std::collections::BTreeMap;

use mushaf_model::Error;
use read_fonts::ps::cff::CffFontRef;
use read_fonts::ps::cs::CommandSink;
use read_fonts::types::{Fixed, GlyphId, Tag};
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
        let data = font
            .table_data(Tag::new(b"CFF2"))
            .ok_or_else(|| err("no CFF2 table (only CFF2 fonts are supported)"))?;
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
}
