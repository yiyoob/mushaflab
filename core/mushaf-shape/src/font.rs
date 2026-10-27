//! Font loading and shaping with harfrust (D3).
//!
//! Every word is shaped on its own (as DigitalKhatt's justifier measures words), right to left,
//! script `Arab`, language `ar`, with monotone-character clusters: a glyph's cluster is the
//! index of the first character of its cluster in the word. Nothing depends on the process
//! locale (HarfBuzz's guessed default language would).
//!
//! harfrust 0.14.0 `Feature::new(tag, value, start..end)` stores an inclusive end that its
//! matcher then treats as exclusive, so single-character ranges never apply (G-019). Ranged
//! features are therefore built field by field with HarfBuzz semantics (end exclusive).

use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;

use harfrust::{
    Buffer, ClusterLevel, Direction, Feature, Language, Script, ShapeOptions, ShaperFont, Tag,
};
use mushaf_model::Error;

/// The parsed font. Construct once; shape through [`FontData::shaper`].
pub struct FontData {
    bytes: Vec<u8>,
    font: harfrust::Font,
    upem: i32,
    /// GDEF glyph class per glyph id (0 when the font has none for it; 3 = mark).
    classes: Vec<u16>,
}

/// One feature value on characters `start..end` of the word (end exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct FeatureRange {
    pub tag: [u8; 4],
    pub value: u32,
    pub start: u32,
    pub end: u32,
}

impl FeatureRange {
    /// The harfrust feature, built directly (G-019).
    pub fn to_harfrust(self) -> Feature {
        Feature {
            tag: Tag::new(&self.tag),
            value: self.value,
            start: self.start,
            end: self.end,
        }
    }
}

/// A positioned glyph in font units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct ShapedGlyph {
    pub glyph: u32,
    /// Index of the first character of the glyph's cluster in the word.
    pub cluster: u32,
    pub x_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

impl FontData {
    pub fn new(bytes: Vec<u8>) -> Result<Self, Error> {
        let font = harfrust::Font::new(bytes.clone(), 0)
            .ok_or_else(|| Error::new("font: cannot parse"))?;
        let upem = i32::from(font.units_per_em());
        let classes = glyph_classes(&bytes, font.num_glyphs())?;
        Ok(FontData {
            bytes,
            font,
            upem,
            classes,
        })
    }

    /// GDEF glyph class (1 base, 2 ligature, 3 mark, 4 component, 0 none).
    pub fn glyph_class(&self, glyph: u32) -> u16 {
        self.classes.get(glyph as usize).copied().unwrap_or(0)
    }

    /// The font's glyph for a character, if its cmap has one.
    pub fn nominal_glyph(&self, ch: char) -> Option<u32> {
        self.font
            .charmap()
            .map_unicode(u32::from(ch))
            .map(|g| g.to_u32())
    }

    pub fn units_per_em(&self) -> i32 {
        self.upem
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn shaper(&self) -> Shaper<'_> {
        Shaper {
            font: &self.font,
            shaper: ShaperFont::new(&self.font),
            cache: RefCell::new(HashMap::new()),
            stats: RefCell::new(CacheStats::default()),
        }
    }
}

/// A word's shaping state (characters, feature ranges) and its shaped glyphs.
type WordCache = HashMap<(Vec<char>, Vec<FeatureRange>), Vec<ShapedGlyph>>;

/// Shaping results are cached per shaper, keyed by the word's whole shaping state: its
/// characters and its feature ranges (T3.6). Shaping is pure, so the cache cannot change any
/// output; the map is only looked up, never iterated.
pub struct Shaper<'a> {
    font: &'a harfrust::Font,
    shaper: ShaperFont<'a, 'a>,
    cache: RefCell<WordCache>,
    stats: RefCell<CacheStats>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl Shaper<'_> {
    /// The font's glyph for a character, if its cmap has one.
    pub fn nominal_glyph(&self, ch: char) -> Option<u32> {
        self.font
            .charmap()
            .map_unicode(u32::from(ch))
            .map(|g| g.to_u32())
    }

    pub fn cache_stats(&self) -> CacheStats {
        *self.stats.borrow()
    }

    pub fn shape(
        &self,
        text: &[char],
        features: &[FeatureRange],
    ) -> Result<Vec<ShapedGlyph>, Error> {
        let key = (text.to_vec(), features.to_vec());
        if let Some(hit) = self.cache.borrow().get(&key) {
            self.stats.borrow_mut().hits += 1;
            return Ok(hit.clone());
        }
        let out = self.shape_uncached(text, features)?;
        self.stats.borrow_mut().misses += 1;
        self.cache.borrow_mut().insert(key, out.clone());
        Ok(out)
    }

    fn shape_uncached(
        &self,
        text: &[char],
        features: &[FeatureRange],
    ) -> Result<Vec<ShapedGlyph>, Error> {
        let mut buffer = Buffer::new();
        for (i, ch) in text.iter().enumerate() {
            buffer.push(u32::from(*ch), index(i)?);
        }
        buffer.set_direction(Direction::RightToLeft);
        buffer.set_script(Some(Script::ARABIC));
        buffer.set_language(Some(
            Language::from_str("ar").map_err(|_| Error::new("language tag"))?,
        ));
        buffer.set_cluster_level(ClusterLevel::MonotoneCharacters);
        let feats: Vec<Feature> = features.iter().map(|f| f.to_harfrust()).collect();
        harfrust::shape(
            &self.shaper,
            &mut buffer,
            ShapeOptions::new().features(&feats),
        )
        .map_err(|e| Error::new(format!("shape: {e:?}")))?;
        Ok(buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(i, p)| ShapedGlyph {
                glyph: i.glyph_id,
                cluster: i.cluster,
                x_advance: p.x_advance,
                x_offset: p.x_offset,
                y_offset: p.y_offset,
            })
            .collect())
    }

    /// Sum of advances, font units.
    pub fn width(&self, text: &[char], features: &[FeatureRange]) -> Result<i32, Error> {
        Ok(self
            .shape(text, features)?
            .iter()
            .map(|g| g.x_advance)
            .sum())
    }
}

fn glyph_classes(bytes: &[u8], n: u32) -> Result<Vec<u16>, Error> {
    use read_fonts::TableProvider;
    let font = read_fonts::FontRef::new(bytes).map_err(|_| Error::new("font: cannot parse"))?;
    let Ok(gdef) = font.gdef() else {
        return Ok(vec![0; n as usize]);
    };
    let Some(Ok(defs)) = gdef.glyph_class_def() else {
        return Ok(vec![0; n as usize]);
    };
    Ok((0..n)
        .map(|g| defs.get(read_fonts::types::GlyphId::new(g)))
        .collect())
}

fn index(i: usize) -> Result<u32, Error> {
    u32::try_from(i).map_err(|_| Error::new("word too long"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranged_features_keep_harfbuzz_semantics() {
        let f = FeatureRange {
            tag: *b"cv01",
            value: 2,
            start: 3,
            end: 4,
        }
        .to_harfrust();
        assert_eq!((f.start, f.end, f.value), (3, 4, 2));
        assert_eq!(f.tag, Tag::new(b"cv01"));
    }
}
